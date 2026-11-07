//! feature: `Option::take`
//! section: Option
//! disposition: easy
//! note: At opt-level 1 rustc calls it on a `&mut Option`, which the compiler doesn't yet write out.
//! input: 5 -1
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
    for _ in 0..2 {
        let a = inp();
        let mut o = maybe(a);
        let t = o.take();
        out(t.unwrap_or(0));
        out(o.is_none() as i16);
    }
}
