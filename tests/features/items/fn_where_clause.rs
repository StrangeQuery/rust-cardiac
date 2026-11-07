//! feature: A `where` clause on a generic function
//! section: Functions
//! disposition: easy
//! note: rustc writes the MIR of each instance (`f::<i16>`); compiling it as an ordinary function is what the compiler already does for non-generic ones.
//! input: 7 2
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
fn diff<T>(a: T, b: T) -> T where T: core::ops::Sub<Output = T> + Copy { a - b }
pub fn main() {
    let a = inp();
    let b = inp();
    out(diff(a, b));
    out(diff(b, a));
}
