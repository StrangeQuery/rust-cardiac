#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// A u8 is never negative: only the test above the arms.
#[inline(never)]
fn byte(b: u8) -> i16 {
    match b {
        0 => 3,
        1 => 1,
        2 => 4,
        3 => -1,
        4 => 5,
        5 => 9,
        6 => 2,
        7 => 6,
        _ => 0,
    }
}

/// A dense match on a u8, with values above the arms.
pub fn main() {
    let mut d: u8 = 0;
    while d < 12 {
        out(byte(d));
        d += 1;
    }
}
