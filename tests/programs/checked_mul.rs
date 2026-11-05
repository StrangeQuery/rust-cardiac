#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// `checked_mul`: the overflow-checked multiply and a test of its flag, `Some` within a card, as natively.
pub fn main() {
    let mut v: i16 = -9;
    while v < 12 {
        out(v.checked_mul(-7).unwrap_or(-600));
        v += 4;
    }
}
