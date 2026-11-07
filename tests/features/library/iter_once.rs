//! feature: `core::iter::once(x)` chained
//! section: Iterators
//! disposition: demand
//! input: 100 1 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    let b = inp();
    let c = inp();
    out(core::iter::once(a).chain(b..c).sum::<i16>());
}
