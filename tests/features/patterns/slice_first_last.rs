//! feature: `[first, .., last]` on an array
//! section: Slice patterns
//! input: 1 2 3 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let t = [inp(), inp(), inp(), inp()];
    let [first, .., last] = t;
    out(first);
    out(last);
}
