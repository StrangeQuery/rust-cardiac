//! feature: `Ord::clamp` on `bool`
//! section: Comparison (cmp, Ordering)
//! disposition: easy
//! note: At opt-level 1 rustc calls `Ord::clamp` on `bool`, as it calls `max` and `min`.
//! input: 0 1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp() > 0;
    let lo = inp() > 0;
    out(a.clamp(lo, true) as i16);
}
