use image::ImageFormat;
use qoi::Pixel;
use rayon::prelude::*;
use std::ffi::{c_void, CString};
use std::fs::File;
use std::io::{BufWriter, Seek, SeekFrom};
use std::num::NonZeroUsize;
use std::os::raw::c_char;
use std::path::PathBuf;
use std::pin::Pin;
use std::time::{Duration, Instant};

extern "C" {
    fn qoi_write(filename: *const c_char, data: *const u8, desc: *const u8) -> i32;
    fn qoi_read(filename: *const c_char, desc: *mut u8, channels: i32) -> *mut u8;
}

#[repr(C)]
#[allow(non_camel_case_types)]
struct qoi_desc {
    width: u32,
    height: u32,
    channels: u8,
    colorspace: u8,
}

#[derive(Debug)]
struct Results {
    file: PathBuf,
    png_size: usize,
    qoi_size: usize,
    image_decode_time: f64,
    image_encode_time: f64,
    qoi_c_encode_time: f64,
    qoi_c_decode_time: f64,
    qoi_rs_encode_time: f64,
    qoi_rs_decode_time: f64,
}

fn main() {
    let out_dir = "./img/output".to_string();
    let _ = std::fs::remove_dir_all(&out_dir);
    std::fs::create_dir(&out_dir).unwrap();

    let mut images = Vec::new();
    read_dir(PathBuf::from("./img/images"), &mut images);

    println!("PROCESSING {} FILES...", images.len());

    let results: Vec<Results> = images
        .par_iter()
        .map(|file| {
            let name = file.file_name().unwrap().to_string_lossy().to_string();
            let c_file = (out_dir.clone() + "/c_" + &name).replace(".png", ".qoi");
            let rs_file = (out_dir.clone() + "/r_" + &name).replace(".png", ".qoi");
            let c_file = CString::new(c_file).unwrap();

            println!("\t{}", name);

            // Load the raw PNG file
            let png_size = { File::open(file).unwrap().seek(SeekFrom::End(0)).unwrap() as usize };

            // Decode the PNG file
            let start = Instant::now();
            let img = image::open(file).unwrap();
            let image_decode_time = (Instant::now() - start).as_secs_f64();

            // Encode the PNG file
            let mut writer = BufWriter::new(File::create(file).unwrap());
            let start = Instant::now();
            img.write_to(&mut writer, ImageFormat::Png).unwrap();
            let image_encode_time = (Instant::now() - start).as_secs_f64();
            drop(writer);

            // Get the image size
            let img = img.to_rgba8();
            let w = img.width() as usize;
            let h = img.height() as usize;
            let pixels: Vec<Pixel> = img.pixels().map(|p| p.0.into()).collect();

            let pin = Pin::new(img);
            let mut desc = qoi_desc {
                width: w as u32,
                height: h as u32,
                channels: 4,
                colorspace: 0,
            };
            let desc_ptr = &desc as *const qoi_desc as *const u8;
            let desc_mut_ptr = &mut desc as *mut qoi_desc as *mut u8;

            // Encode the image using the C QOI encoder
            let start = Instant::now();
            let len = unsafe { qoi_write(c_file.as_ptr(), pin.as_ptr(), desc_ptr) };
            if len == 0 {
                panic!("FAILED TO ENCODE: {:?} ({}x{})", c_file, w, h);
            }
            let qoi_c_encode_time = (Instant::now() - start).as_secs_f64();
            let qoi_size = len as usize;

            // Decode the image using the C QOI decoder
            let start = Instant::now();
            let ptr = unsafe { qoi_read(c_file.as_ptr(), desc_mut_ptr, 4) };
            let qoi_c_decode_time = (Instant::now() - start).as_secs_f64();
            unsafe { libc::free(ptr as *mut c_void) };
            assert_eq!(desc.width as usize, w);
            assert_eq!(desc.height as usize, h);

            // Encode the image using the Rust QOI encoder
            let start = Instant::now();
            let writer = BufWriter::new(File::create(&rs_file).unwrap());
            let qoi_rs_size = qoi::encode(
                NonZeroUsize::new(w).unwrap(),
                NonZeroUsize::new(h).unwrap(),
                pixels.into_iter(),
                writer,
            )
            .unwrap();
            let qoi_rs_encode_time = (Instant::now() - start).as_secs_f64();
            assert_eq!(qoi_size, qoi_rs_size);

            // Decode the image using the Rust QOI decoder
            let start = Instant::now();
            let mut _data: Vec<Pixel> = Vec::with_capacity(w * h);
            let (ww, hh) = qoi::decode_file_into_vec(&rs_file, &mut _data).unwrap();
            assert_eq!(ww, w);
            assert_eq!(hh, h);
            let qoi_rs_decode_time = (Instant::now() - start).as_secs_f64();

            Results {
                file: file.to_path_buf(),
                png_size,
                qoi_size,
                image_decode_time,
                image_encode_time,
                qoi_c_encode_time,
                qoi_c_decode_time,
                qoi_rs_encode_time,
                qoi_rs_decode_time,
            }
        })
        .collect();

    let png_size = results.iter().map(|r| r.png_size).sum::<usize>() / results.len();
    let qoi_size = results.iter().map(|r| r.qoi_size).sum::<usize>() / results.len();
    let image_encode_time: f64 = results.iter().map(|r| r.image_encode_time).sum();
    let image_decode_time: f64 = results.iter().map(|r| r.image_decode_time).sum();
    let qoi_c_encode_time: f64 = results.iter().map(|r| r.qoi_c_encode_time).sum();
    let qoi_c_decode_time: f64 = results.iter().map(|r| r.qoi_c_decode_time).sum();
    let qoi_r_encode_time: f64 = results.iter().map(|r| r.qoi_rs_encode_time).sum();
    let qoi_r_decode_time: f64 = results.iter().map(|r| r.qoi_rs_decode_time).sum();

    //for result in &results {
    //    println!("{:#?}", result);
    //}

    let r = results.len() as f64;

    println!("AVERAGE FILE SIZE:");
    let p = (qoi_size as f64) / (png_size as f64);
    println!("\tpng ...... {} kb", png_size / 1000);
    println!("\tqoi ...... {} kb ({:.2}x larger)", qoi_size / 1000, p);

    println!("AVERAGE ENCODE TIME:");
    let i = (image_encode_time / r) * 1000.0;
    let c = (qoi_c_encode_time / r) * 1000.0;
    let r = (qoi_r_encode_time / r) * 1000.0;
    let cp = image_encode_time / qoi_c_encode_time;
    let rp = image_encode_time / qoi_r_encode_time;
    println!("\timage .... {:.2} ms", i);
    println!("\tc ........ {:.2} ms ({:.2}x faster)", c, cp);
    println!("\trust ..... {:.2} ms ({:.2}x faster)", r, rp);

    println!("AVERAGE DECODE TIME:");
    let i = (image_decode_time / r) * 1000.0;
    let c = (qoi_c_decode_time / r) * 1000.0;
    let r = (qoi_r_decode_time / r) * 1000.0;
    let cp = image_decode_time / qoi_c_decode_time;
    let rp = image_decode_time / qoi_r_decode_time;
    println!("\timage .... {:.2} ms", i);
    println!("\tc ........ {:.2} ms ({:.2}x faster)", c, cp);
    println!("\trust ..... {:.2} ms ({:.2}x faster)", r, rp);
}

fn read_dir(dir: PathBuf, images: &mut Vec<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries {
            let entry = entry.unwrap();
            if let Some(ext) = entry.path().extension() {
                if ext == "png" {
                    images.push(entry.path());
                }
            } else {
                read_dir(entry.path(), images);
            }
        }
    }
}
