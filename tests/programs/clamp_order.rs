#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// clamp asserts min <= max, as Rust's does: the third pair is out of order and panics.
pub fn main() {
    let lo: [i16; 3] = [0, -5, 6];
    let hi: [i16; 3] = [9, 5, 2];
    let mut i = 0usize;
    while i < 3 {
        out(7i16.clamp(lo[i], hi[i]));
        i += 1;
    }
}
