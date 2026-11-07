//! feature: `array.sort_unstable()` on four elements
//! section: Arrays and slices
//! disposition: demand
//! note: core's sort is far bigger than 96 cells would allow; a small insertion sort written out fits.
//! input: 3 -1 4 1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let t: [i16; 4] = [inp(), inp(), inp(), inp()];
    let mut t = t;
    t.sort_unstable();
    for i in 0..t.len() {
        out(t[i]);
    }
}
