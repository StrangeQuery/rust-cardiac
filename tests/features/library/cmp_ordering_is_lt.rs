//! feature: `Ordering::is_lt` and `is_ge`
//! section: Comparison (cmp, Ordering)
//! disposition: easy
//! note: An `Ordering` is -1, 0 or 1 (an `i8`), and rustc's three-way compare is two tests, as `x == k` already is.
//! input: 1 2 3 2
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    for _ in 0..2 {
        let a = inp();
        let b = inp();
        let o = a.cmp(&b);
        out(o.is_lt() as i16);
        out(o.is_ge() as i16);
    }
}
