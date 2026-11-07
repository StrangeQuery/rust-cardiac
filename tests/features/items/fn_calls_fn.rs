//! feature: A non-inlined function calling another
//! section: Functions
//! input: 7
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn inc(x: i16) -> i16 { x + 1 }
#[inline(never)]
fn inc2(x: i16) -> i16 { inc(inc(x)) }
pub fn main() {
    out(inc2(inp()));
}
