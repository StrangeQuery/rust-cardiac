//! feature: `array.iter().sum()`
//! section: Arrays and slices
//! disposition: easy
//! note: The same loop as `for i in 0..N` adding `t[i]`.
//! input: 3 -1 4 1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let t: [i16; 4] = [inp(), inp(), inp(), inp()];
    out(t.iter().sum::<i16>());
}
