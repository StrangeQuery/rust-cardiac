//! feature: Tuple struct pattern (`Pair(a, b)`)
//! section: Tuple struct patterns
//! input: 7 2
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
struct Pair(i16, i16);
pub fn main() {
    let p = Pair(inp(), inp());
    let Pair(a, b) = p;
    out(a - b);
}
