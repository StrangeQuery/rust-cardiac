//! feature: `&x` in a parameter (`fn f(&x: &i16)`)
//! section: Reference patterns
//! input: 6 -2
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn twice(&x: &i16) -> i16 { x + x }
pub fn main() {
    for _ in 0..2 {
        let v = inp();
        out(twice(&v));
    }
}
