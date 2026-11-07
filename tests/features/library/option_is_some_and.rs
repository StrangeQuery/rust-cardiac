//! feature: `Option::is_some_and`
//! section: Option
//! disposition: easy
//! note: At opt-level 1 rustc calls it, as it does `map`, whose call the compiler already writes out.
//! input: 5 2
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
        let o = maybe(a);
        out(o.is_some_and(|x| x > 3) as i16);
    }
}
