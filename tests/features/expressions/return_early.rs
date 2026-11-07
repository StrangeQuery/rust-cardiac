//! feature: An early `return` from a function
//! section: Return expressions
//! input: -4 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn f(a: i16) -> i16 {
    if a < 0 { return 0; }
    a * 3
}
pub fn main() {
    out(f(inp()));
    out(f(inp()));
}
