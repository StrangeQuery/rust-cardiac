//! feature: A slice `&[i16]` passed to a function and read in a loop
//! section: Slice types
//! disposition: demand
//! note: A slice is an address and a length, two cells, read the way arrays by reference already are.
//! input: 3 4 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn total(s: &[i16]) -> i16 {
    let mut t = 0;
    let mut i = 0;
    while i < s.len() {
        t += s[i];
        i += 1;
    }
    t
}
pub fn main() {
    let a = [inp(), inp(), inp()];
    out(total(&a));
}
