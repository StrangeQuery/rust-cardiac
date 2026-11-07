//! feature: `array.swap(i, j)`
//! section: Arrays and slices
//! disposition: easy
//! note: On an array it is two indexed reads and two writes, with bounds checks.
//! input: 3 -1 4 1 0 2
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let t: [i16; 4] = [inp(), inp(), inp(), inp()];
    let mut t = t;
    let i = inp() as usize;
    let j = inp() as usize;
    t.swap(i, j);
    for i in 0..t.len() {
        out(t[i]);
    }
}
