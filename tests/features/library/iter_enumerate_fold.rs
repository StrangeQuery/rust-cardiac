//! feature: `t.iter().enumerate()` in an adaptor chain
//! section: Iterators
//! disposition: demand
//! input: 5 7 9
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let t: [i16; 3] = [inp(), inp(), inp()];
    out(t.iter().enumerate().filter(|&(_, &x)| x > 6).map(|(i, _)| i as i16).sum::<i16>());
}
