#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

const T: [i16; 4] = [10, 11, 12, 13];

/// Without overflow checks a u8 wraps: 0 - 1 is 255, 60 + 200 is 4, 60 * 5 is 44. A cell does not, and
/// kept -1, 260 and 300.
#[inline(never)]
fn show(b: u8) {
    out((b - 1) as i16);
    out((b + 200) as i16);
    out((b * 5) as i16);
}

/// An index made from a u8 that wrapped: 0 - 1 is 255, out of bounds, and the bounds check panics. With -1 it
/// passed, and read the cell before the table.
#[inline(never)]
fn index(b: u8) {
    out(T[(b - 1) as usize]);
}

pub fn main() {
    show(3);
    show(60);
    show(0);
    index(1);
    index(0);
}
