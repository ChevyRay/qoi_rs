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

    /// Pack the pixel into a 32-bit RGBA integer.
    #[inline]
    pub fn pack(self) -> u32 {
        (self.r as u32) << 24 | (self.g as u32) << 16 | (self.b as u32) << 8 | (self.a as u32)
    }

    /// Unpack the pixel from a 32-bit RGBA integer.
    #[inline]
    pub fn unpack(packed: u32) -> Self {
        Self {
            r: (packed >> 24) as u8,
            g: (packed >> 16) as u8,
            b: (packed >> 8) as u8,
            a: packed as u8,
        }
    }
}

impl From<u32> for Pixel {
    #[inline]
    fn from(val: u32) -> Self {
        Self::unpack(val)
    }
}

impl Into<u32> for Pixel {
    #[inline]
    fn into(self) -> u32 {
        self.pack()
    }
}

impl From<(u8, u8, u8, u8)> for Pixel {
    #[inline]
    fn from((r, g, b, a): (u8, u8, u8, u8)) -> Self {
        Self { r, g, b, a }
    }
}

impl Into<(u8, u8, u8, u8)> for Pixel {
    #[inline]
    fn into(self) -> (u8, u8, u8, u8) {
        (self.r, self.g, self.b, self.a)
    }
}

impl From<[u8; 4]> for Pixel {
    #[inline]
    fn from([r, g, b, a]: [u8; 4]) -> Self {
        Self { r, g, b, a }
    }
}

impl Into<[u8; 4]> for Pixel {
    #[inline]
    fn into(self) -> [u8; 4] {
        [self.r, self.g, self.b, self.a]
    }
}
