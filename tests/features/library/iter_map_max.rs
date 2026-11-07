//! feature: `.map(...).max()` and `.min()`
//! section: Iterators
//! disposition: demand
//! input: 3 -1 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let t: [i16; 3] = [inp(), inp(), inp()];
    out(t.iter().map(|&x| 10 - x).max().unwrap());
    out(t.iter().map(|&x| 10 - x).min().unwrap());
}
