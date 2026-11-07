//! feature: `array.iter().any(...)`
//! section: Arrays and slices
//! disposition: easy
//! note: The same loop as `for i in 0..N` with an early return.
//! input: 3 -1 4 1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let t: [i16; 4] = [inp(), inp(), inp(), inp()];
    out(t.iter().any(|&x| x < 0) as i16);
    out(t.iter().any(|&x| x > 50) as i16);
}
