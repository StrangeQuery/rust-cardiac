//! feature: `array.iter().fold(...)`
//! section: Arrays and slices
//! disposition: easy
//! note: The same loop as `for i in 0..N` with the closure's body inside it.
//! input: 0 3 -1 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let t: [i16; 4] = [inp(), inp(), inp(), inp()];
    out(t.iter().fold(0, |acc, &x| acc * 10 + x));
}
