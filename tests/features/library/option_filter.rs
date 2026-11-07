//! feature: `Option::filter`
//! section: Option
//! disposition: easy
//! note: rustc calls it at both opt-levels; the predicate takes a reference to the payload, which is a scalar.
//! input: 5 2 -1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn maybe(a: i16) -> Option<i16> {
    if a > 0 { Some(a) } else { None }
}
pub fn main() {
    for _ in 0..3 {
        let a = inp();
        let o = maybe(a);
        out(o.filter(|&x| x > 3).unwrap_or(0));
    }
}
