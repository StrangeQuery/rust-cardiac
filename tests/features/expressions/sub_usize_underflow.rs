//! feature: `a - b` on `usize` below 0: panics, or wraps with checks off
//! section: Arithmetic operators
//! input: 3 7
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp() as usize;
    let b = inp() as usize;
    let c = a - b;
    out((c > 5) as i16);
}
