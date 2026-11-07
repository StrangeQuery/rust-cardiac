//! feature: `#[unsafe(no_mangle)]` on a helper the program calls
//! section: Linkage attributes
//! disposition: easy
//! input: 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn twice(a: i16) -> i16 { a + a }
pub fn main() {
    out(twice(inp()));
}
