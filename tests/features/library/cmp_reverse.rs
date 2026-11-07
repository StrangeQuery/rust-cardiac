//! feature: `core::cmp::Reverse`
//! section: Comparison (cmp, Ordering)
//! disposition: demand
//! note: `Reverse` is a one-field struct from core whose comparisons swap their operands.
//! input: 1 2 3 2
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
use core::cmp::Reverse;
pub fn main() {
    for _ in 0..2 {
        let a = inp();
        let b = inp();
        out((Reverse(a) < Reverse(b)) as i16);
        out(core::cmp::max(Reverse(a), Reverse(b)).0);
    }
}
