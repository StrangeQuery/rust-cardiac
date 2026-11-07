//! feature: Several trait bounds (`T: Copy + PartialOrd + Add`)
//! section: Generic parameters
//! disposition: easy
//! note: rustc writes the MIR of each instance (`f::<i16>`); compiling it as an ordinary function is what the compiler already does for non-generic ones.
//! input: 5 3 9
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
use core::ops::Add;
fn clamp_add<T: Copy + PartialOrd + Add<Output = T>>(a: T, b: T, hi: T) -> T { let s = a + b; if s > hi { hi } else { s } }
pub fn main() {
    let a = inp();
    let b = inp();
    let h = inp();
    out(clamp_add(a, b, h));
    out(clamp_add(a, a, 100));
}
