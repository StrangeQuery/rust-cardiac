//! feature: A const generic parameter (`fn f<const N: usize>`)
//! section: Functions
//! disposition: easy
//! note: rustc writes the MIR of each instance (`f::<i16>`); compiling it as an ordinary function is what the compiler already does for non-generic ones.
//! input: 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
fn times<const N: usize>(x: i16) -> i16 {
    let mut s = 0;
    for _ in 0..N { s += x; }
    s
}
pub fn main() {
    let v = inp();
    out(times::<3>(v));
    out(times::<7>(v));
}
