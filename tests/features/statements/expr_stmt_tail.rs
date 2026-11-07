//! feature: a block whose value is its tail expression
//! section: Expression statements
//! input: 7
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
fn f(x: i16) -> i16 {
    let y = x + 1;
    y * 2
}
pub fn main() {
    out(f(inp()));
}
