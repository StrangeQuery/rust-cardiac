//! feature: `use` of a function from a module
//! section: Use declarations
//! input: 8
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
mod m {
    pub fn neg(x: i16) -> i16 { -x }
}
use m::neg;
pub fn main() {
    out(neg(inp()));
}
