//! feature: `slice.len()` on a slice of run-time length
//! section: Arrays and slices
//! disposition: demand
//! note: A slice is an address and a length; the compiler has arrays of known length only.
//! input: 3 2
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let t: [i16; 4] = [1, 2, 3, 4];
    for _ in 0..2 {
        let n = inp() as usize;
        let s = &t[..n];
        out(s.len() as i16);
    }
}
