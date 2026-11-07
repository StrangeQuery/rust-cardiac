//! feature: `core::hint::black_box`
//! section: Hints (core::hint)
//! disposition: easy
//! note: On CARDIAC it can simply be the value itself.
//! input: 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    out(core::hint::black_box(a) + 1);
}
