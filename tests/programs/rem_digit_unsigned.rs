#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// `% 2` and `% 5` of a u8 and a usize: never negative, the last digit and a pass or two of the loop.
#[inline(never)]
fn rem_unsigned(b: u8, u: usize) {
    out((b % 2) as i16);
    out((b % 5) as i16);
    out((u % 2) as i16);
    out((u % 5) as i16);
}

const BYTES: [u8; 5] = [0, 1, 9, 128, 255];
const WIDE: [usize; 5] = [0, 1, 9, 504, 999];

pub fn main() {
    for i in 0..5 {
        rem_unsigned(BYTES[i], WIDE[i]);
    }
}
