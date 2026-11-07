//! feature: A bare `return;` from a function that returns nothing
//! section: Return expressions
//! input: -4 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn show(a: i16) {
    if a < 0 { return; }
    out(a);
}
pub fn main() {
    show(inp());
    show(inp());
    out(0);
}
