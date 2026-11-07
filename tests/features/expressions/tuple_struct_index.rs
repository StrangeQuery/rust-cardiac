//! feature: A tuple struct's fields, `p.0`, `p.1`
//! section: Tuple and tuple indexing expressions
//! input: 6 7
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
struct Pair(i16, i16);
pub fn main() {
    let mut p = Pair(inp(), inp());
    p.0 += p.1;
    out(p.0);
    out(p.1);
}
