//! feature: `&x` passed to a function taking `&i16`
//! section: Borrow operators
//! input: 4 6
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn sum(a: &i16, b: &i16) -> i16 { *a + *b }
pub fn main() {
    let a = inp();
    let b = inp();
    out(sum(&a, &b));
}
