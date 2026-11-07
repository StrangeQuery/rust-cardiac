//! feature: A sub-slice `&t[1..3]`, indexed
//! section: Slice types
//! disposition: demand
//! input: 3 4 5 6 1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let t = [inp(), inp(), inp(), inp()];
    let s: &[i16] = &t[1..3];
    let i = inp() as usize;
    out(s[i]);
    out(s.len() as i16);
}
