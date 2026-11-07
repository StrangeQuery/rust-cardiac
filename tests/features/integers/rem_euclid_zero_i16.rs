//! feature: `a.rem_euclid(b)` with `b` zero panics
//! section: Euclidean division
//! input: 7 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    let b = inp();
    out(a.rem_euclid(b));
}
