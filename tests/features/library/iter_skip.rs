//! feature: `(a..b).skip(n)`
//! section: Iterators
//! disposition: demand
//! input: 2 6 2
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    let b = inp();
    let n = inp() as usize;
    out((a..b).skip(n).sum::<i16>());
}
