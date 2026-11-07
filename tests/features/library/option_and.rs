//! feature: `Option::and`
//! section: Option
//! disposition: easy
//! note: At opt-level 1 rustc calls it rather than inlining it, and the compiler doesn't yet write out that call.
//! input: 1 2 1 -2 -1 2 -1 -2
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
    for _ in 0..4 {
        let a = inp();
        let b = inp();
        let o = maybe(a);
        let p = maybe(b);
        out(o.and(p).unwrap_or(0));
    }
}
