//! feature: A tuple struct built with a call, `Pair(a, b)`
//! section: Struct expressions
//! input: 3 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
struct Pair(i16, i16);
#[inline(never)]
fn swap(p: Pair) -> Pair { Pair(p.1, p.0) }
pub fn main() {
    let p = swap(Pair(inp(), inp()));
    out(p.0);
    out(p.1);
}
