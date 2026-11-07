//! feature: `(a..b).count()`
//! section: Iterators
//! disposition: easy
//! note: On a range it is `b - a` (or 0).
//! input: 2 6
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    let b = inp();
    out((a..b).count() as i16);
}
