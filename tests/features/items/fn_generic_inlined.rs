//! feature: A generic function rustc inlines
//! section: Functions
//! disposition: easy
//! note: rustc writes the MIR of each instance (`f::<i16>`); compiling it as an ordinary function is what the compiler already does for non-generic ones.
//! input: 3 8
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
fn bigger<T: PartialOrd>(a: T, b: T) -> T { if a > b { a } else { b } }
pub fn main() {
    let a = inp();
    let b = inp();
    out(bigger(a, b));
    out(bigger(a < 5, b < 5) as i16);
}
