# QOI - The “Quite OK Image” format

This is a Rust encoder and decoder for Dominic Szablewski's QOI format
for fast, lossless image compression.

> *QOI encodes and decodes images in a lossless format. An encoded QOI image is
usually around 10-30% larger than a decently optimized PNG image.*
> 
> *QOI outperforms simpler PNG encoders in compression ratio and performance. QOI
images are typically 20% smaller than PNGs written with stbi_image but 10%
larger than with libpng. Encoding is 25-50x faster and decoding is 3-4x faster
than stbi_image or libpng.*

About the format: [https://phoboslab.org/log/2021/11/qoi-fast-lossless-image-compression](https://phoboslab.org/log/2021/11/qoi-fast-lossless-image-compression)

This was ported from Dominic's [original C code](https://github.com/phoboslab/qoi),
but modified to be much more idiomatic in Rust.