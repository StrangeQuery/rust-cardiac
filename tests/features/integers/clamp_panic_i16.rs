//! feature: `a.clamp(lo, hi)` with `lo > hi` panics
//! section: Min, max and clamp
//! input: 5 10 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    let lo = inp();
    let hi = inp();
    out(a.clamp(lo, hi));
}
