//! feature: `slice.copy_from_slice(...)`
//! section: Arrays and slices
//! disposition: demand
//! note: Subslices with run-time bounds need slices, which the compiler doesn't have.
//! input: 3 -1 4 1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let t: [i16; 4] = [inp(), inp(), inp(), inp()];
    let mut u: [i16; 4] = [0; 4];
    u[1..3].copy_from_slice(&t[2..4]);
    for i in 0..u.len() {
        out(u[i]);
    }
}
