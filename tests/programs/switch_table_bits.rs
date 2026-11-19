#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// A u8's low three bits: 0 to 7, every value an arm.
#[inline(never)]
fn low_bits(b: u8) -> i16 {
    match b & 7 {
        0 => 10,
        1 => 11,
        2 => 12,
        3 => 13,
        4 => 14,
        5 => 15,
        6 => 16,
        _ => 17,
    }
}

/// A dense match on a mask of a u8's bits.
pub fn main() {
    let mut d: u8 = 0;
    while d < 20 {
        out(low_bits(d + d + d));
        d += 1;
    }
}
