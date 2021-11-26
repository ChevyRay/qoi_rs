use downloader::download::Download;
use downloader::Downloader;
use git2::Repository;
use std::fs::File;
use std::path::Path;
use tar::Archive;

fn main() {
    // Clone the original C encoder/decoder repository
    let c_path = "./qoi_c";
    let _ = std::fs::remove_dir_all(c_path);
    let _ = Repository::clone("https://github.com/phoboslab/qoi", c_path).unwrap();

    // Compile and link the C files
    cc::Build::new()
        .file("./src/qoi.c")
        .flag("-Wno-unsequenced")
        .compile("qoi");

    // Download the PNG suite tarball
    {
        if !File::open("./img/images.tar").is_ok() {
            let _ = std::fs::create_dir("./img");
            let mut downloader = Downloader::builder()
                .download_folder(Path::new("./img"))
                .build()
                .unwrap();
            let _ = downloader
                .download(&[Download::new(
                    "https://phoboslab.org/files/qoibench/images.tar",
                )])
                .unwrap();
        }
    }

    let mut archive = Archive::new(File::open("./img/images.tar").unwrap());
    archive.unpack("./img").unwrap();
}
