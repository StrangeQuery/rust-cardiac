//! feature: `(a..b).chain(c..d)`
//! section: Iterators
//! disposition: demand
//! input: 1 3 10 12
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    let b = inp();
    let c = inp();
    let d = inp();
    out((a..b).chain(c..d).sum::<i16>());
}
