# QOI - The “Quite OK Image” format

This is a Rust encoder and decoder for Dominic Szablewski's
[QOI format](https://phoboslab.org/log/2021/11/qoi-fast-lossless-image-compression)
for fast, lossless image compression.

This was ported from Dominic's [original C code](https://github.com/phoboslab/qoi),
but modified to be much more idiomatic in Rust.

> *QOI encodes and decodes images in a lossless format. An encoded QOI image is
usually around 10-30% larger than a decently optimized PNG image.*
>
> *QOI outperforms simpler PNG encoders in compression ratio and performance. QOI
images are typically 20% smaller than PNGs written with stbi_image but 10%
larger than with libpng. Encoding is 25-50x faster and decoding is 3-4x faster
than stbi_image or libpng.*

## Usage

You can call `encode()` to encode an image. You supply it with an iterator
of `Pixel` values, and a writer to output to.

```rust
use std::num::NonZeroUsize;
use qoi::Pixel;

// Create a 512x256 transparent image here to demonstrate
let width = 512;
let height = 256;
let mut pixels: Vec<Pixel> = Vec::new();
pixels.resize_with(width * height, || Pixel::transparent());

// Encode the image and write it to a file
let file = File::create("my_image.qoi").unwrap();
qoi::encode(
    NonZeroUsize::new(width).unwrap(),
    NonZeroUsize::new(height).unwrap(),
    pixels.into_iter(),
    BufWriter::new(file),
)
.unwrap();
```

There are several helpful decode functions, here's the inverse of the above:

```rust
use qoi::Pixel;

let mut pixels: Vec<Pixel> = Vec::new();
qoi::decode_file_into_vec("my_image.qoi", &mut pixels).unwrap();
```