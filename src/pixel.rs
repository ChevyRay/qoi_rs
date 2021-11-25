// An RGBA pixel.
#[repr(C)]
#[derive(Default, Copy, Clone, Eq, PartialEq, Debug)]
pub struct Pixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Pixel {
    /// A transparent pixel (0, 0, 0, 0)
    #[inline]
    pub const fn transparent() -> Self {
        Self::rgba(0, 0, 0, 0)
    }

    /// Create a new pixel.
    #[inline]
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Create an RGB pixel with a full (255) alpha channel.
    #[inline]
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::rgba(r, g, b, 255)
    }

    /// Hash the pixel's RGBA components together. This is used
    /// by the encoder/decoder to create storage indices for the
    /// running lookup table.
    #[inline]
    pub(crate) const fn hash(self) -> u8 {
        self.r ^ self.g ^ self.b ^ self.a
    }
}
