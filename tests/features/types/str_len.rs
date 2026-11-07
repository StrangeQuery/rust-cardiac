//! feature: `.len()` of one of two string literals chosen by an input
//! section: Textual types
//! disposition: demand
//! note: A `&str` is a slice: an address and a length.
//! input: 1 -1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let mut k = 0;
    while k < 2 {
        let s = if inp() > 0 { "abc" } else { "hello" };
        out(s.len() as i16);
        k += 1;
    }
}
