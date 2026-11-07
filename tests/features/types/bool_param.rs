//! feature: A `bool` passed to and returned from a function
//! section: Boolean type
//! input: 3 -3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn flip(b: bool) -> bool {
    !b
}
pub fn main() {
    let a = flip(inp() > 0);
    let b = flip(inp() > 0);
    out(a as i16);
    out(b as i16);
}
