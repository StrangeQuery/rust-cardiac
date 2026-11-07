//! feature: `bool::then`
//! section: bool and char
//! disposition: easy
//! note: At opt-level 1 rustc calls it with a closure; it is `if b { Some(f()) } else { None }`.
//! input: 5 -1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    for _ in 0..2 {
        let a = inp();
        out((a > 0).then(|| a * 2).unwrap_or(0));
    }
}
