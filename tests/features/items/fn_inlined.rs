//! feature: A small helper rustc inlines, called twice
//! section: Functions
//! input: 5 6
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
fn add3(x: i16) -> i16 { x + 3 }
pub fn main() {
    out(add3(inp()));
    out(add3(inp()));
}
