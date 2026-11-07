//! feature: `#[track_caller]` on a function that may panic
//! section: Code generation attributes
//! note: The second call panics in `unwrap()`; the panic's location is the caller's line.
//! input: 5 -3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[track_caller]
#[inline(never)]
fn checked(x: i16) -> i16 {
    let o = if x < 0 { None } else { Some(x) };
    o.unwrap() + 1
}
pub fn main() {
    out(checked(inp()));
    out(checked(inp()));
}
