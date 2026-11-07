//! feature: An `extern "C" fn` defined in Rust and called from Rust
//! section: External blocks
//! disposition: easy
//! input: 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
extern "C" fn add1(a: i16) -> i16 { a + 1 }
pub fn main() {
    out(add1(inp()));
}
