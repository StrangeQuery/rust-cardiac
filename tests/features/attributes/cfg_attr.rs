//! feature: `#[cfg_attr(all(), inline(never))]` and `#[cfg_attr(any(), ...)]`
//! section: Conditional compilation
//! input: 5 7
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[cfg_attr(all(), inline(never))]
fn twice(a: i16) -> i16 { a + a }
#[cfg_attr(any(), inline(always))]
fn thrice(a: i16) -> i16 { a + a + a }
pub fn main() {
    let a = inp();
    let b = inp();
    out(twice(a));
    out(thrice(b));
}
