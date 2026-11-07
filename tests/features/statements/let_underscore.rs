//! feature: `let _ = e;` evaluates `e` and drops it
//! section: Let statements
//! input: 7 3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let _ = inp();
    out(inp());
}
