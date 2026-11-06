#![no_std]
#[inline(never)]
pub fn out(_v: u8) {}

/// overflowing_mul on a u8, as rustc inlines it at opt-level 2: the product wraps, which past a card can't be
/// worked out from a cell, so it is refused.
#[inline(never)]
fn show(a: u8) {
    let (v, o) = a.overflowing_mul(3);
    out(v);
    out(o as u8);
}

pub fn main() {
    show(100);
}
