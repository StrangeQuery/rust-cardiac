//! feature: A closure capturing a variable by reference
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
    let k = inp();
    let f = |a: i16| a * 2 + k;
    out(f(inp()));
    out(f(k));
}
