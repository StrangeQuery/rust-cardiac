//! feature: `(a..b).sum()`
//! section: Loop expressions
//! disposition: easy
//! input: 1 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    let b = inp();
    let s: i16 = (a..b).sum();
    out(s);
}
