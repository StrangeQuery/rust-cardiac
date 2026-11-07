//! feature: `match a.cmp(&b)` on `Ordering`
//! section: Comparison (cmp, Ordering)
//! disposition: easy
//! note: An `Ordering` is -1, 0 or 1 (an `i8`), and rustc's three-way compare is two tests, as `x == k` already is.
//! input: 1 2 2 2 3 2
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
use core::cmp::Ordering;
pub fn main() {
    for _ in 0..3 {
        let a = inp();
        let b = inp();
        out(match a.cmp(&b) {
            Ordering::Less => -1,
            Ordering::Equal => 0,
            Ordering::Greater => 1,
        });
    }
}
