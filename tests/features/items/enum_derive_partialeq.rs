//! feature: `==` on an enum with `#[derive(PartialEq)]`
//! section: Enumerations
//! input: 1 1 2
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[derive(PartialEq, Clone, Copy)]
enum C { R, G, B }
fn pick(v: i16) -> C { match v { 0 => C::R, 1 => C::G, _ => C::B } }
pub fn main() {
    let a = pick(inp());
    let b = pick(inp());
    let c = pick(inp());
    out((a == b) as i16);
    out((a == c) as i16);
}
