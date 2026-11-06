#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 {
    0
}

/// i16::MAX as a value, which no card holds: refused as a constant past a card, not a crash.
pub fn main() {
    let a = inp();
    out((i16::MAX - a) / 100);
}
