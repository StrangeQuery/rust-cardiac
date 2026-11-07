//! feature: A function of six parameters, not inlined
//! section: Functions
//! input: 1 2 3 4 5 6
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn f(a: i16, b: i16, c: i16, d: i16, e: i16, g: i16) -> i16 { a - b + c - d + e - g }
pub fn main() {
    out(f(inp(), inp(), inp(), inp(), inp(), inp()));
}
