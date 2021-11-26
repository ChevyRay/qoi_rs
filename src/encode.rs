use crate::{consts::*, Error, Pixel};
use std::io::{Seek, SeekFrom, Write};
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
    W: Write + Seek,
{
    // Get our parameters into useful form
    let width = width.get();
    let height = height.get();

    let start_pos = output.stream_position()?;

    // Write the file type marker and image size
    output.write(&MAGIC)?;
    output.write(&(width as u16).to_le_bytes())?;
    output.write(&(height as u16).to_le_bytes())?;

    // This will contain the amount of bytes in the data block, but
    // we don't know it yet, so just fill it with a temp value and
    // store the position so we can populate it later
    let size_pos = output.stream_position()?;
    output.write(&i32::to_le_bytes(0))?;
    let data_pos = output.stream_position()?;

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
                output.write(&[RUN_8 | (run as u8)])?;
            } else {
                // If it's a long run, encode it in 2 bytes (RUN_16)
                run -= 33;
                output.write(&[RUN_16 | ((run >> 8) as u8), run as u8])?;
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
                output.write(&[INDEX | index_u8])?;
            } else {
                // If the pixel is different than the lookup value, overwrite it
                lookup[index] = px;

                // Get the difference between this and the previous pixel
                let dr = (px.r as i16) - (prev.r as i16);
                let dg = (px.g as i16) - (prev.g as i16);
                let db = (px.b as i16) - (prev.b as i16);
                let da = (px.a as i16) - (prev.a as i16);

                // If the difference is small enough, we'll encode the pixel as a difference
                if dr > -16
                    && dr < 17
                    && dg > -16
                    && dg < 17
                    && db > -16
                    && db < 17
                    && da > -16
                    && da < 17
                {
                    if da == 0 && dr > -2 && dr < 3 && dg > -2 && dg < 3 && db > -2 && db < 3 {
                        // If the difference can be encoded in 2 bits for each channel,
                        // pack all 3 differences into one byte (DIFF_8)
                        output.write(&[
                            DIFF_8 | ((((dr + 1) << 4) | (dg + 1) << 2 | (db + 1)) as u8)
                        ])?;
                    } else if da == 0
                        && dr > -16
                        && dr < 17
                        && dg > -8
                        && dg < 9
                        && db > -8
                        && db < 9
                    {
                        // If the red difference fits in 5 bits and the green/blue fit in 4 bits,
                        // pack all the differences together into two bytes. (DIFF_16)
                        output.write(&[
                            DIFF_16 | ((dr + 15) as u8),
                            (((dg + 7) << 4) | (db + 7)) as u8,
                        ])?;
                    } else {
                        // If each channel requires 5 bits to store its difference, then we pack
                        // them all into 3 bytes (DIFF_24)
                        output.write(&[
                            DIFF_24 | (((dr + 15) >> 1) as u8),
                            (((dr + 15) << 7) | ((dg + 15) << 2) | ((db + 15) >> 3)) as u8,
                            (((db + 15) << 5) | (da + 15)) as u8,
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
                    output.write(&chunk[..i])?;
                }
            }
        }

        // Store the pixel and move onto the next. We track the current pixel's index
        // as well so that if the last pixel is part of a run, we can finish the run
        prev = px;
    }

    // Mark the end of the data block with 4 empty bytes
    output.write(&[0, 0, 0, 0])?;

    // Go back and fill the size value with the size of the data block,
    // then return the stream back to the end position
    let end_pos = output.stream_position()?;
    let size = (end_pos - data_pos) as i32;
    output.seek(SeekFrom::Start(size_pos))?;
    output.write(&size.to_le_bytes())?;
    output.seek(SeekFrom::Start(end_pos))?;

    Ok((end_pos - start_pos) as usize)
}
