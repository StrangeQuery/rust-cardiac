//! feature: An `FnMut` closure that changes a captured counter
//! section: Closure types
//! disposition: easy
//! note: At opt-level 1 rustc calls the closure rather than inlining it; the closure is a struct of references and a function, both of which the compiler already handles.
//! input: 3 4 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let mut total = 0;
    let mut add = |x: i16| total += x;
    add(inp());
    add(inp());
    add(inp());
    out(total);
}
