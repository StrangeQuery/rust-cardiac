//! feature: `[T; N]::map`
//! section: Arrays and slices
//! disposition: demand
//! input: 3 -1 4 1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let t: [i16; 4] = [inp(), inp(), inp(), inp()];
    let u = t.map(|x| x * 2);
    for i in 0..u.len() {
        out(u[i]);
    }
}
