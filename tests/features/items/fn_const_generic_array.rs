//! feature: A const generic array length (`fn sum<const N: usize>(t: &[i16; N])`)
//! section: Functions
//! disposition: easy
//! note: rustc writes the MIR of each instance (`f::<i16>`); compiling it as an ordinary function is what the compiler already does for non-generic ones.
//! input: 1 2 3 4 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
fn sum<const N: usize>(t: &[i16; N]) -> i16 {
    let mut s = 0;
    for i in 0..N { s += t[i]; }
    s
}
pub fn main() {
    let a = [inp(), inp(), inp()];
    let b = [inp(), inp()];
    out(sum(&a));
    out(sum(&b));
}
