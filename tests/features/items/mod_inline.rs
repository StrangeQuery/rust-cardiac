//! feature: An inline `mod` with a `pub fn`
//! section: Modules
//! input: 4 -3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
mod maths {
    pub fn twice(x: i16) -> i16 { x + x }
}
pub fn main() {
    out(maths::twice(inp()));
    out(maths::twice(inp()));
}
