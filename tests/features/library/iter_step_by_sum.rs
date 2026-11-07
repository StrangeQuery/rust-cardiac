//! feature: `(a..b).step_by(k).sum()`
//! section: Iterators
//! disposition: demand
//! input: 0 10 3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    let b = inp();
    let k = inp() as usize;
    out((a..b).step_by(k).sum::<i16>());
}
