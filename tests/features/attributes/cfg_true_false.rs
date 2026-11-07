//! feature: `#[cfg(all())]` and `#[cfg(any())]` on functions
//! section: Conditional compilation
//! input: 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[cfg(all())]
fn pick(a: i16) -> i16 { a + 1 }
#[cfg(any())]
fn pick(a: i16) -> i16 { a + 100 }
pub fn main() {
    out(pick(inp()));
}
