//! feature: A closure called where it's made
//! section: Closure expressions
//! disposition: easy
//! note: At opt-level 1 rustc calls the closure rather than inlining it; the call is a method on a struct of its captures.
//! input: 3 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let add = |a: i16, b: i16| a + b;
    let x = inp();
    let y = inp();
    out(add(x, y));
    out(add(y, 1));
}
