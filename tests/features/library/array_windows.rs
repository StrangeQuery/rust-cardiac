//! feature: `array.windows(2)`
//! section: Arrays and slices
//! disposition: demand
//! note: Each window is a slice in an `Option`, which the compiler doesn't have.
//! input: 3 -1 4 1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let t: [i16; 4] = [inp(), inp(), inp(), inp()];
    for w in t.windows(2) {
        out(w[1] - w[0]);
    }
}
