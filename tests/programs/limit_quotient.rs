#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 {
    0
}

/// i16::MAX as a value, divided by 1000: refused as a constant past a card. A quotient by 1000 of what a card
/// holds is 0, so the difference is never read; refused all the same, where natively this outputs 32.
pub fn main() {
    let a = inp();
    out((i16::MAX - a) / 1000);
}
