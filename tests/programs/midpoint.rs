#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 {
    0
}

/// i16::midpoint, which rustc inlines at opt-level 2 as a sum in i32: past a card for 999 and 998, so refused.
pub fn main() {
    let a = inp();
    let b = inp();
    out(a.midpoint(b));
}
