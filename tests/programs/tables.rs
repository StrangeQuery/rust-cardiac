#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// Named consts from a module, by path: a table and two scalars.
mod tables {
    pub const SQUARES: [i16; 16] = [0, 1, 4, 9, 16, 25, 36, 49, 64, 81, 100, 121, 144, 169, 196, 225];
    pub const STEP: i16 = 5;
    pub const OFFSET: i16 = -7;
}

pub fn main() {
    let squares = tables::SQUARES;
    let mut i = 0;
    while i < 16 { out(squares[i] + tables::OFFSET); i += tables::STEP as usize; }
}
