//! feature: `(a..b).map(...).sum()`
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
    out((a..b).map(|x| x * 2).sum::<i16>());
}
