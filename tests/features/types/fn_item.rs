//! feature: A function item held in a variable and called
//! section: Function item types
//! input: 6
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn double(x: i16) -> i16 {
    x * 2
}
pub fn main() {
    let f = double;
    out(f(inp()));
}
