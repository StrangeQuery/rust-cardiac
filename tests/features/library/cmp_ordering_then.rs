//! feature: `Ordering::then` (compare by two keys)
//! section: Comparison (cmp, Ordering)
//! disposition: easy
//! note: An `Ordering` is -1, 0 or 1 (an `i8`); `then` is a test of the first for zero.
//! input: 1 5 1 3 1 5 2 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    for _ in 0..2 {
        let a = inp();
        let b = inp();
        let c = inp();
        let d = inp();
        out(a.cmp(&c).then(b.cmp(&d)) as i16);
    }
}
