//! feature: `a % b` with `b` zero panics
//! section: Arithmetic operators
//! input: 7 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    let b = inp();
    out(a % b);
}
