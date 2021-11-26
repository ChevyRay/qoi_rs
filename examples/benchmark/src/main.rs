use image::{DynamicImage, EncodableLayout, GenericImageView, ImageFormat, RgbaImage};
use qoi::Pixel;
use rayon::prelude::*;
use std::ffi::{c_void, CStr, CString, OsStr};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::num::NonZeroUsize;
use std::os::raw::c_char;
use std::path::PathBuf;
use std::pin::Pin;
use std::time::{Duration, Instant};

extern "C" {
    fn qoi_encode(data: *const u8, w: i32, h: i32, channels: i32, out_len: *mut i32) -> *mut u8;
    fn qoi_write(filename: *const c_char, data: *const u8, w: i32, h: i32, channels: i32) -> i32;

    //void *qoi_read(const char *filename, int *out_w, int *out_h, int channels);
    fn qoi_read(filename: *const c_char, w: *mut i32, h: *mut i32, channels: i32) -> *mut u8;
}

#[derive(Debug)]
struct Results {
    file: PathBuf,
    image_decode_time: Duration,
    image_encode_time: Duration,
    image_size: usize,
    qoi_c_encode_time: Duration,
    qoi_c_decode_time: Duration,
    qoi_c_size: usize,
    qoi_rs_encode_time: Duration,
    qoi_rs_decode_time: Duration,
    qoi_rs_size: usize,
}

#[derive(Default)]
struct DummyWriter {
    bytes: Vec<u8>,
    pos: usize,
}
impl DummyWriter {
    fn with_capacity(cap: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(cap),
            pos: 0,
        }
    }
}
impl Write for DummyWriter {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if self.pos == self.bytes.len() {
            self.bytes.extend_from_slice(buf);
            self.pos += buf.len();
        } else {
            for &b in buf {
                if self.pos < self.bytes.len() {
                    unsafe { *self.bytes.get_unchecked_mut(self.pos) = b };
                } else {
                    self.bytes.push(b);
                }
                self.pos += 1;
            }
        }
        Ok(buf.len())
    }

    #[inline]
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
impl Seek for DummyWriter {
    #[inline]
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        match pos {
            SeekFrom::Start(pos) => {
                self.pos = pos as usize;
            }
            SeekFrom::End(pos) => {
                self.pos = ((self.bytes.len() as i64) + pos) as usize;
            }
            SeekFrom::Current(pos) => {
                if pos > 0 {
                    self.pos += pos as usize;
                } else {
                    self.pos -= (-pos) as usize;
                }
            }
        }
        Ok(self.pos as u64)
    }

    fn stream_position(&mut self) -> std::io::Result<u64> {
        Ok(self.pos as u64)
    }
}

fn main() {
    let out_dir = "./img/output".to_string();
    let _ = std::fs::remove_dir_all(&out_dir);
    std::fs::create_dir(&out_dir).unwrap();

    let start = Instant::now();

    let mut images = Vec::new();
    read_dir(PathBuf::from("./img/images"), &mut images);
    let results: Vec<Results> = images
        .par_iter()
        .map(|file| {
            //let dir = file.parent().unwrap().to_string_lossy().to_string();
            let name = file.file_name().unwrap().to_string_lossy().to_string();
            let c_file = (out_dir.clone() + "/c_" + &name).replace(".png", ".qoi");
            let rs_file = (out_dir.clone() + "/r_" + &name).replace(".png", ".qoi");
            let c_file = CString::new(c_file).unwrap();

            // Load the raw PNG file
            let mut bytes = Vec::with_capacity(8192 * 8192 * 4);
            File::open(file).unwrap().read_to_end(&mut bytes);
            let image_size = bytes.len();

            // Decode the PNG file
            let start = Instant::now();
            let img = image::load_from_memory_with_format(&bytes, ImageFormat::Png).unwrap();
            let image_decode_time = Instant::now() - start;

            // Encode the PNG file
            //let mut writer = DummyWriter::with_capacity(8192 * 8192 * 4);
            let mut writer = BufWriter::new(File::create(file).unwrap());
            let start = Instant::now();
            img.write_to(&mut writer, ImageFormat::Png).unwrap();
            let image_encode_time = Instant::now() - start;
            drop(writer);

            // Get the image size
            let img = img.to_rgba8();
            let w = img.width() as usize;
            let h = img.height() as usize;
            let pixels: Vec<Pixel> = img.pixels().map(|p| p.0.into()).collect();

            let pin = Pin::new(img);

            // Encode the image using the C QOI encoder
            let start = Instant::now();
            let len = unsafe { qoi_write(c_file.as_ptr(), pin.as_ptr(), w as i32, h as i32, 4) };
            if len == 0 {
                println!("FAILED TO ENCODE: {:?} ({}x{})", c_file, w, h);
            }
            let qoi_c_encode_time = Instant::now() - start;
            let qoi_c_size = len as usize;

            // Decode the image using the C QOI decoder
            let start = Instant::now();
            let (mut ww, mut hh) = (0, 0);
            let ptr = unsafe { qoi_read(c_file.as_ptr(), &mut ww, &mut hh, 4) };
            if ww == 0 {
                //println!("FAILED TO DECODE: {} -> {:?}", c_file, ptr);
            }
            //assert_eq!(ww as usize, w);
            //assert_eq!(hh as usize, h);
            let qoi_c_decode_time = Instant::now() - start;
            unsafe { libc::free(ptr as *mut c_void) };

            // Encode the image using the Rust QOI encoder
            let start = Instant::now();
            let mut writer = BufWriter::new(File::create(&rs_file).unwrap());
            let qoi_rs_size = qoi::encode(
                NonZeroUsize::new(w).unwrap(),
                NonZeroUsize::new(h).unwrap(),
                pixels.into_iter(),
                writer,
            )
            .unwrap();
            let qoi_rs_encode_time = Instant::now() - start;

            // Decode the image using the Rust QOI decoder
            let start = Instant::now();
            let mut _data: Vec<Pixel> = Vec::with_capacity(w * h);
            let (ww, hh) = qoi::decode_file_into_vec(&rs_file, &mut _data).unwrap();
            assert_eq!(ww, w);
            assert_eq!(hh, h);
            let qoi_rs_decode_time = Instant::now() - start;

            // Free the qoi data
            //unsafe { libc::free(ptr as *mut c_void) };

            Results {
                file: file.to_path_buf(),
                image_decode_time,
                image_encode_time,
                image_size,
                qoi_c_encode_time,
                qoi_c_decode_time,
                qoi_c_size,
                qoi_rs_encode_time,
                qoi_rs_decode_time,
                qoi_rs_size,
            }
        })
        .collect();

    let mut image_encode_time: Duration = results.iter().map(|r| r.image_encode_time).sum();
    let mut image_decode_time: Duration = results.iter().map(|r| r.image_decode_time).sum();
    let mut qoi_c_encode_time: Duration = results.iter().map(|r| r.qoi_c_encode_time).sum();
    let mut qoi_c_decode_time: Duration = results.iter().map(|r| r.qoi_c_decode_time).sum();
    let mut qoi_r_encode_time: Duration = results.iter().map(|r| r.qoi_rs_encode_time).sum();
    let mut qoi_r_decode_time: Duration = results.iter().map(|r| r.qoi_rs_decode_time).sum();

    println!("total time: {:?}", Instant::now() - start);

    println!("image_encode_time: {:?}", image_encode_time);
    println!("image_decode_time: {:?}", image_decode_time);
    println!("qoi_c_encode_time: {:?}", qoi_c_encode_time);
    println!("qoi_c_decode_time: {:?}", qoi_c_decode_time);
    println!("qoi_r_encode_time: {:?}", qoi_r_encode_time);
    println!("qoi_r_decode_time: {:?}", qoi_r_decode_time);
}

fn read_dir(dir: PathBuf, images: &mut Vec<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries {
            let entry = entry.unwrap();
            if let Some(ext) = entry.path().extension() {
                if ext == "png" {
                    //println!("{:?}", entry.path().file_name().unwrap());
                    //println!("{:?}\n\t{:?}", entry.path(), entry.path().parent().unwrap());
                    images.push(entry.path());
                }
            } else {
                read_dir(entry.path(), images);
            }
        }
    }
}
