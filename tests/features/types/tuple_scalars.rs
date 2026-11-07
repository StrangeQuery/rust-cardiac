//! feature: A tuple of `i16`, `bool` and `u8`
//! section: Tuple types
//! input: 5 -3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let t: (i16, bool, u8) = (inp(), inp() > 0, 7);
    out(t.0);
    out(t.1 as i16);
    out(t.2 as i16 + t.0);
}
