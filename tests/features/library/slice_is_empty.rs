//! feature: `slice.is_empty()`
//! section: Arrays and slices
//! disposition: demand
//! note: A slice is an address and a length; the compiler has arrays of known length only.
//! input: 0 2
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
        out(s.is_empty() as i16);
    }
}
