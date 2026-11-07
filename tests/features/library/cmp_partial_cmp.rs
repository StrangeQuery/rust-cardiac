//! feature: `PartialOrd::partial_cmp`
//! section: Comparison (cmp, Ordering)
//! disposition: easy
//! note: An `Option<Ordering>`: once `Ordering` is supported it is a small enum like any other.
//! input: 1 2 3 2
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
use core::cmp::Ordering;
pub fn main() {
    for _ in 0..2 {
        let a = inp();
        let b = inp();
        out((a.partial_cmp(&b) == Some(Ordering::Less)) as i16);
    }
}
