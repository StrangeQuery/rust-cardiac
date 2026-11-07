//! feature: Struct pattern with named fields (`P { x: a, y: b }`)
//! section: Struct patterns
//! input: 3 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
struct P { x: i16, y: i16 }
pub fn main() {
    let p = P { x: inp(), y: inp() };
    let P { x: a, y: b } = p;
    out(a * 10 + b);
}
