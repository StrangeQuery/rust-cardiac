//! feature: `..` in a tuple struct pattern (`T(a, ..)`)
//! section: Rest patterns
//! input: 1 2 3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
struct T(i16, i16, i16);
pub fn main() {
    let t = T(inp(), inp(), inp());
    let T(a, ..) = t;
    out(a);
    out(t.2);
}
