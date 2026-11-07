//! feature: `#[unsafe(export_name = "...")]` on a helper the program calls
//! section: Linkage attributes
//! input: 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[unsafe(export_name = "renamed_helper")]
#[inline(never)]
pub fn helper(a: i16) -> i16 { a + 3 }
pub fn main() {
    out(helper(inp()));
}
