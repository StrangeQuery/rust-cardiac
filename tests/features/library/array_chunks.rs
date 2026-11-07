//! feature: `array.chunks(2)`
//! section: Arrays and slices
//! disposition: demand
//! note: Each chunk is a slice in an `Option`, which the compiler doesn't have.
//! input: 3 -1 4 1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let t: [i16; 4] = [inp(), inp(), inp(), inp()];
    for c in t.chunks(2) {
        out(c[0] + c[1]);
    }
}
