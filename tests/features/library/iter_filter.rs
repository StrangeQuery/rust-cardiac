//! feature: `(a..b).filter(...).sum()`
//! section: Iterators
//! disposition: demand
//! input: 2 6
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    let b = inp();
    out((a..b).filter(|&x| x > 3).sum::<i16>());
}
