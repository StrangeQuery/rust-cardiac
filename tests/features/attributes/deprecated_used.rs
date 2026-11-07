//! feature: `#[deprecated]` on a function the program calls
//! section: Diagnostic attributes
//! input: 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[deprecated(since = "1.0", note = "use new_way")]
fn old_way(a: i16) -> i16 { a + 1 }
#[allow(deprecated)]
pub fn main() {
    out(old_way(inp()));
}
