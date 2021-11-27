use crate::{consts::*, Error, Pixel};
use std::io::Write;
use std::num::NonZeroUsize;

/// Encodes the pixels supplied by the `pixels` iterator into the `output` stream. The iterator is
/// expected to have `width * height` pixels in it. Returns the size of the encoded data.
pub fn encode<I, W>(
    width: NonZeroUsize,
    height: NonZeroUsize,
    mut pixels: I,
    mut output: W,
) -> Result<usize, Error>
where
    I: Iterator<Item = Pixel>,
    W: Write,
{
    // Get our parameters into useful form
    let width = width.get();
    let height = height.get();

    let mut num_bytes = 0;
    let mut write = |buf: &[u8]| {
        num_bytes += buf.len();
        output.write(buf)
    };

    // Write the file header
    write(&MAGIC.to_be_bytes())?;
    write(&(width as u32).to_be_bytes())?;
    write(&(height as u32).to_be_bytes())?;
    write(&[4, 0])?;

    // A running lookup table of previously seen pixels
    let mut lookup = [Pixel::transparent(); 64];
    let mut prev = Pixel::rgba(0, 0, 0, 255);
    let mut run: u16 = 0;
    let num_pixels = width * height;
    let mut count = 0;

    while count < num_pixels {
        count += 1;

        // Get our next pixel, returning an error if the iterator runs dry
        let px = pixels.next().ok_or_else(|| Error::IteratorEmpty)?;

        // If multiple pixels are same in a row, increase the run-length
        if px == prev {
            run += 1;
        }

        // Check if we've got a run going, but we've hit the end of it
        if run > 0 && (run == 0x2020 || px != prev || count == num_pixels) {
            if run < 33 {
                // If it's a short run, encode it in 1 byte (RUN_8)
                run -= 1;
                write(&[RUN_8 | (run as u8)])?;
            } else {
                // If it's a long run, encode it in 2 bytes (RUN_16)
                run -= 33;
                write(&[RUN_16 | ((run >> 8) as u8), run as u8])?;
            }
            run = 0;
        }

        // If this pixel isn't a run
        if px != prev {
            let index_u8 = px.hash() % 64;
            let index = index_u8 as usize;
            if lookup[index] == px {
                // If our pixel is in the lookup table, we can just write an
                // index byte indicating which position in the table it's at
                write(&[INDEX | index_u8])?;
            } else {
                // If the pixel is different than the lookup value, overwrite it
                lookup[index] = px;

                // Get the difference between this and the previous pixel
                let vr = (px.r as i16) - (prev.r as i16);
                let vg = (px.g as i16) - (prev.g as i16);
                let vb = (px.b as i16) - (prev.b as i16);
                let va = (px.a as i16) - (prev.a as i16);

                // If the difference is small enough, we'll encode the pixel as a difference
                if vr > -17
                    && vr < 16
                    && vg > -17
                    && vg < 16
                    && vb > -17
                    && vb < 16
                    && va > -17
                    && va < 16
                {
                    if va == 0 && vr > -3 && vr < 2 && vg > -3 && vg < 2 && vb > -3 && vb < 2 {
                        // If the difference can be encoded in 2 bits for each channel,
                        // pack all 3 differences into one byte (DIFF_8)
                        write(&[DIFF_8 | ((((vr + 2) << 4) | (vg + 2) << 2 | (vb + 2)) as u8)])?;
                    } else if va == 0
                        && vr > -17
                        && vr < 16
                        && vg > -9
                        && vg < 8
                        && vb > -9
                        && vb < 8
                    {
                        // If the red difference fits in 5 bits and the green/blue fit in 4 bits,
                        // pack all the differences together into two bytes. (DIFF_16)
                        write(&[
                            DIFF_16 | ((vr + 16) as u8),
                            (((vg + 8) << 4) | (vb + 8)) as u8,
                        ])?;
                    } else {
                        // If each channel requires 5 bits to store its difference, then we pack
                        // them all into 3 bytes (DIFF_24)
                        write(&[
                            DIFF_24 | (((vr + 16) >> 1) as u8),
                            (((vr + 16) << 7) | ((vg + 16) << 2) | ((vb + 16) >> 3)) as u8,
                            (((vb + 16) << 5) | (va + 16)) as u8,
                        ])?;
                    }
                } else {
                    // This pixel is wholly unique, so we have to encode it. But instead of encoding
                    // the whole thing, we can check each of the RGBA channels and see if it is
                    // different than the previous pixel's. If it is, then we flag that channel's bit
                    // in the tag byte, and append the channel's color value.
                    let mut chunk = [COLOR, 0, 0, 0, 0];
                    let mut i = 1;
                    if px.r != prev.r {
                        chunk[0] |= 8;
                        chunk[i] = px.r;
                        i += 1;
                    }
                    if px.g != prev.g {
                        chunk[0] |= 4;
                        chunk[i] = px.g;
                        i += 1;
                    }
                    if px.b != prev.b {
                        chunk[0] |= 2;
                        chunk[i] = px.b;
                        i += 1;
                    }
                    if px.a != prev.a {
                        chunk[0] |= 1;
                        chunk[i] = px.a;
                        i += 1;
                    }
                    write(&chunk[..i])?;
                }
            }
        }

        // Store the pixel and move onto the next. We track the current pixel's index
        // as well so that if the last pixel is part of a run, we can finish the run
        prev = px;
    }

    // Mark the end of the data block with 4 empty bytes
    write(&[0, 0, 0, 0])?;

    // Return the total amount of bytes that were encoded
    Ok(num_bytes)
}
