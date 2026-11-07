//! feature: A function taking and returning `usize`, not inlined
//! section: Functions
//! input: 30 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn f(a: usize, b: usize) -> usize { a + b * 2 }
pub fn main() {
    let a = inp() as usize;
    let b = inp() as usize;
    out(f(a, b) as i16);
}
