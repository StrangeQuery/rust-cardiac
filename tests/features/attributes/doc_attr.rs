//! feature: `#[doc = "..."]`, `///` comments and `#[doc(hidden)]`
//! section: Documentation attributes
//! input: 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
/// Adds one.
#[doc = "More text."]
#[doc(hidden)]
fn inc(a: i16) -> i16 { a + 1 }
pub fn main() {
    out(inc(inp()));
}
