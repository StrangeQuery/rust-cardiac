//! feature: `core::iter::repeat(x).take(n)`
//! section: Iterators
//! disposition: demand
//! input: 7 3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    let n = inp() as usize;
    out(core::iter::repeat(a).take(n).sum::<i16>());
}
