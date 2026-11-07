//! feature: Several `?` in one function: the first `None` returns
//! section: The question mark operator
//! input: 1 2 -1 2
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn pos(x: i16) -> Option<i16> { if x > 0 { Some(x) } else { None } }
#[inline(never)]
fn sum2(a: i16, b: i16) -> Option<i16> { Some(pos(a)? + pos(b)?) }
pub fn main() {
    out(sum2(inp(), inp()).unwrap_or(-1));
    out(sum2(inp(), inp()).unwrap_or(-1));
}
