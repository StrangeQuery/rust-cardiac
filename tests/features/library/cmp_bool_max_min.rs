//! feature: `Ord::max` and `min` on `bool`
//! section: Comparison (cmp, Ordering)
//! disposition: easy
//! note: At opt-level 1 rustc calls `Ord::max` on `bool`; it is `a || b` and `a && b`.
//! input: 1 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp() > 0;
    let b = inp() > 0;
    out(Ord::max(a, b) as i16);
    out(Ord::min(a, b) as i16);
}
