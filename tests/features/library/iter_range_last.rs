//! feature: `(a..b).last()`
//! section: Iterators
//! disposition: easy
//! note: On a range it is `b - 1`, or `None` when the range is empty.
//! input: 2 6
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    let b = inp();
    out((a..b).last().unwrap_or(-1));
}
