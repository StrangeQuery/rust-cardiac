//! feature: A tuple struct
//! section: Structs
//! input: 6 7
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[derive(Clone, Copy)]
struct Pair(i16, i16);
#[inline(never)]
fn swap(p: Pair) -> Pair { Pair(p.1, p.0) }
pub fn main() {
    let p = swap(Pair(inp(), inp()));
    out(p.0);
    out(p.1);
}
