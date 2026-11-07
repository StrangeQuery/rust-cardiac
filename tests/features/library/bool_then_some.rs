//! feature: `bool::then_some`
//! section: bool and char
//! disposition: easy
//! note: At opt-level 1 rustc calls it; it is `if b { Some(x) } else { None }`.
//! input: 5 -1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    for _ in 0..2 {
        let a = inp();
        out((a > 0).then_some(a).unwrap_or(0));
    }
}
