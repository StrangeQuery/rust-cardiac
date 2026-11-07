//! feature: Nested struct patterns
//! section: Struct patterns
//! input: 1 2 3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
struct In { a: i16, b: i16 }
struct Out { i: In, c: i16 }
#[inline(never)]
fn make(a: i16, b: i16, c: i16) -> Out { Out { i: In { a, b }, c } }
pub fn main() {
    let o = make(inp(), inp(), inp());
    let Out { i: In { a, b }, c } = o;
    out(a + b + c);
    out(b);
}
