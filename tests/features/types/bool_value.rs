//! feature: A `bool` made from a comparison, cast to `i16`
//! section: Boolean type
//! input: 5 -5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a: bool = inp() > 0;
    let b: bool = inp() > 0;
    out(a as i16);
    out(b as i16);
}
