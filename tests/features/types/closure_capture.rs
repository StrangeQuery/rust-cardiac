//! feature: A closure that captures a variable, called where it is made
//! section: Closure types
//! disposition: easy
//! note: At opt-level 1 rustc calls the closure rather than inlining it; the closure is a struct of references and a function, both of which the compiler already handles.
//! input: 5 3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let k = inp();
    let add = |x: i16| x + k;
    out(add(inp()));
}
