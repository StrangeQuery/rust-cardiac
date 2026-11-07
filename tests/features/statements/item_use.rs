//! feature: a `use` declaration inside a function
//! section: Item declarations
//! input: 7 3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    use core::cmp::max;
    let a = inp();
    let b = inp();
    out(max(a, b));
}
