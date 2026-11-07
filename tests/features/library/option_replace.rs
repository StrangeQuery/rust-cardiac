//! feature: `Option::replace`
//! section: Option
//! disposition: easy
//! note: At opt-level 1 rustc calls it on a `&mut Option`, which the compiler doesn't yet write out.
//! input: 8 5 -1
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
    let c = inp();
    for _ in 0..2 {
        let x = inp();
        let mut o = maybe(x);
        let old = o.replace(c);
        out(old.unwrap_or(0));
        out(o.unwrap_or(0));
    }
}
