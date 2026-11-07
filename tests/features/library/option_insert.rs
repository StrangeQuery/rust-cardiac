//! feature: `Option::insert`
//! section: Option
//! disposition: easy
//! note: At opt-level 1 rustc calls it on a `&mut Option`, which the compiler doesn't yet write out.
//! input: -1 8
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
    let a = inp();
    let c = inp();
    let mut o = maybe(a);
    let r = o.insert(c);
    *r += 1;
    out(o.unwrap_or(0));
}
