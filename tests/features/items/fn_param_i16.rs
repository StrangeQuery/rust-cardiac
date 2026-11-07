//! feature: A function taking and returning `i16`, not inlined
//! section: Functions
//! input: 12 -5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn f(a: i16, b: i16) -> i16 { a - b }
pub fn main() {
    let a = inp();
    let b = inp();
    out(f(a, b));
    out(f(b, a));
}
