//! feature: A function taking and returning `bool`, not inlined
//! section: Functions
//! input: 3 -3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn neg(x: i16) -> bool { x < 0 }
#[inline(never)]
fn either(a: bool, b: bool) -> bool { a || b }
pub fn main() {
    let a = neg(inp());
    let b = neg(inp());
    out(a as i16);
    out(b as i16);
    out(either(a, b) as i16);
}
