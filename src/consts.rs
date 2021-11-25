pub const MAGIC: [u8; 4] = [b'q', b'o', b'i', b'f'];

pub const INDEX: u8 = 0x0;
pub const RUN_8: u8 = 0x40;
pub const RUN_16: u8 = 0x60;
pub const DIFF_8: u8 = 0x80;
pub const DIFF_16: u8 = 0xc0;
pub const DIFF_24: u8 = 0xe0;
pub const COLOR: u8 = 0xf0;

pub const MASK_2: u8 = 0xc0;
pub const MASK_3: u8 = 0xe0;
pub const MASK_4: u8 = 0xf0;
