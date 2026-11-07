//! feature: `array.reverse()`
//! section: Arrays and slices
//! disposition: easy
//! note: On an array of known length it is a short loop of swaps.
//! input: 3 -1 4 1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let t: [i16; 4] = [inp(), inp(), inp(), inp()];
    let mut t = t;
    t.reverse();
    for i in 0..t.len() {
        out(t[i]);
    }
}
