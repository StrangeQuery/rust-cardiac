//! feature: `t.iter().rev().fold(...)`
//! section: Iterators
//! disposition: demand
//! input: 1 2 3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let t: [i16; 3] = [inp(), inp(), inp()];
    out(t.iter().rev().fold(0, |acc, &x| acc * 10 + x));
}
