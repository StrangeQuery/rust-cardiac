//! feature: `use ... as` to rename
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
use m::neg as flip;
pub fn main() {
    out(flip(inp()));
}
