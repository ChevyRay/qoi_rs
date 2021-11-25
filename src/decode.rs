use crate::{consts::*, Error, Pixel};
use std::io::Read;
use std::mem::MaybeUninit;

#[inline]
fn read<R: Read, const N: usize>(input: &mut R) -> Result<[u8; N], Error> {
    let mut bytes: [u8; N] = unsafe { MaybeUninit::uninit().assume_init() };
    input.read_exact(&mut bytes)?;
    Ok(bytes)
}

#[inline]
fn read_u8<R: Read>(input: &mut R) -> Result<u8, Error> {
    Ok(read::<R, 1>(input)?[0])
}

#[inline]
fn read_u16<R: Read>(input: &mut R) -> Result<u16, Error> {
    Ok(u16::from_le_bytes(read::<R, 2>(input)?))
}

#[inline]
fn read_i32<R: Read>(input: &mut R) -> Result<i32, Error> {
    Ok(i32::from_le_bytes(read::<R, 4>(input)?))
}

/// Decode the image encoded in the bytes provided by `input`. The return value
/// is the image's `width`, `height`, and an iterator to parse the actual pixel
/// data. If you just want to read the image size, you can ignore the iterator.
///
/// The amount of pixels on a successful decode will always be `width * height`,
/// so you can use those values to pre-allocate your pixel buffer if you want.
pub fn decode<R>(mut input: R) -> Result<(usize, usize, Pixels<R>), Error>
where
    R: Read,
{
    // Parse the magic filetype marker.
    let magic = read::<R, 4>(&mut input)?;
    if magic != MAGIC {
        return Err(Error::InvalidFileTypeMarker(magic));
    }

    // Parse the image size
    let width = read_u16(&mut input)? as usize;
    let height = read_u16(&mut input)? as usize;
    if width == 0 || height == 0 {
        return Err(Error::NoImageSize);
    }

    // Parse the size of our data block
    let data_len = read_i32(&mut input)? as usize;
    if data_len == 0 {
        return Err(Error::NoImageData);
    }

    // Return the image info and an iterator to decode the pixels
    Ok((
        width,
        height,
        Pixels {
            input,
            remaining: width * height,
            px: Pixel::rgba(0, 0, 0, 255),
            run: 0,
            lookup: [Pixel::transparent(); 64],
        },
    ))
}

/// An iterator that parses pixels from the encoded image's data block.
///
/// Since this iterator parses the data as it goes, it iterates over
/// `Result` values that will carry an error if the parser fails.
pub struct Pixels<R> {
    input: R,
    remaining: usize,
    px: Pixel,
    run: u16,
    lookup: [Pixel; 64],
}

impl<R> Pixels<R>
where
    R: Read,
{
    /// Iterate over only the successfully parsed pixels. This iterator
    /// will panic if the parser encounters an error.
    pub fn unwrapped(&mut self) -> Unwrapped<'_, R> {
        Unwrapped { pixels: self }
    }

    /// Iterate over only the successfully parsed pixels. This iterator
    /// will silently end if the parser encounters an error.
    pub fn ok(&mut self) -> Okay<'_, R> {
        Okay { pixels: self }
    }

    fn parse(&mut self) -> Result<Pixel, Error> {
        // If we've got a run, just count it down and return the same pixel again
        if self.run > 0 {
            self.run -= 1;
        } else {
            // Read the first byte, which will contain the tag
            let b1 = read_u8(&mut self.input)?;

            if (b1 & MASK_2) == INDEX {
                // If the pixel is indexed, get the value from the lookup table
                self.px = self.lookup[(b1 ^ INDEX) as usize];
            } else if (b1 & MASK_3) == RUN_8 {
                // If the pixel is a short run, get the run length
                self.run = (b1 & 0x1f) as u16;
            } else if (b1 & MASK_3) == RUN_16 {
                // If the pixel is a long run, get the run length
                let b2 = read_u8(&mut self.input)?;
                self.run = ((((b1 & 0x1f) as u16) << 8) | (b2 as u16)) + 32;
            } else if (b1 & MASK_2) == DIFF_8 {
                self.px.r = self.px.r.wrapping_add(((b1 >> 4) & 0x03).wrapping_sub(1));
                self.px.g = self.px.g.wrapping_add(((b1 >> 2) & 0x03).wrapping_sub(1));
                self.px.b = self.px.b.wrapping_add((b1 & 0x03).wrapping_sub(1));
            } else if (b1 & MASK_3) == DIFF_16 {
                let b2 = read_u8(&mut self.input)?;
                self.px.r = self.px.r.wrapping_add((b1 & 0x1f).wrapping_sub(15));
                self.px.g = self.px.g.wrapping_add((b2 >> 4).wrapping_sub(7));
                self.px.b = self.px.b.wrapping_add((b2 & 0x0f).wrapping_sub(7));
            } else if (b1 & MASK_4) == DIFF_24 {
                let [b2, b3] = read::<R, 2>(&mut self.input)?;
                self.px.r = self
                    .px
                    .r
                    .wrapping_add((((b1 & 0x0f) << 1) | (b2 >> 7)).wrapping_sub(15));
                self.px.g = self.px.g.wrapping_add(((b2 & 0x7c) >> 2).wrapping_sub(15));
                self.px.b = self
                    .px
                    .b
                    .wrapping_add((((b2 & 0x03) << 3) | ((b3 & 0xe0) >> 5)).wrapping_sub(15));
                self.px.a = self.px.a.wrapping_add((b3 & 0x1f).wrapping_sub(15));
            } else if (b1 & MASK_4) == COLOR {
                if (b1 & 8) != 0 {
                    self.px.r = read_u8(&mut self.input)?;
                }
                if (b1 & 4) != 0 {
                    self.px.g = read_u8(&mut self.input)?;
                }
                if (b1 & 2) != 0 {
                    self.px.b = read_u8(&mut self.input)?;
                }
                if (b1 & 1) != 0 {
                    self.px.a = read_u8(&mut self.input)?;
                }
            }

            // Put the new pixel into the lookup table
            self.lookup[(self.px.hash() % 64) as usize] = self.px;
        }

        self.remaining -= 1;
        Ok(self.px)
    }
}

impl<R> Iterator for Pixels<R>
where
    R: Read,
{
    type Item = Result<Pixel, Error>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        (self.remaining > 0).then(|| {
            let result = self.parse();

            // If we get an error while parsing, end the iterator
            if result.is_err() {
                self.remaining = 0;
            }

            result
        })
    }
}

/// An iterator that parses pixels from the encoded image's data block.
/// If the parser encounters an error, this iterator will panic.
pub struct Unwrapped<'a, R> {
    pixels: &'a mut Pixels<R>,
}

impl<'a, R> Iterator for Unwrapped<'a, R>
where
    R: Read,
{
    type Item = Pixel;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.pixels.next().and_then(|p| Some(p.unwrap()))
    }
}

/// An iterator that parses pixels from the encoded image's data block.
/// If the parser fails, this iterator will discard the error and finish.
/// In this event, it is up to the user to check if the correct amount
/// of pixels were parsed.
pub struct Okay<'a, R> {
    pixels: &'a mut Pixels<R>,
}

impl<'a, R> Iterator for Okay<'a, R>
where
    R: Read,
{
    type Item = Pixel;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.pixels.next().and_then(|p| p.ok())
    }
}