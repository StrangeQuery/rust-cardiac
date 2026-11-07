#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 {
    0
}

/// A literal past a card, in a difference divided by 1000: refused, where natively this outputs 1.
pub fn main() {
    let a = inp();
    out((1500 - a) / 1000);
}
