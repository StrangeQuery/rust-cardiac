//! feature: A `const fn` called with a value read at run time
//! section: Functions
//! input: 9 -2
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
const fn tri(n: i16) -> i16 { n * 3 + 1 }
pub fn main() {
    out(tri(inp()));
    out(tri(inp()));
}
