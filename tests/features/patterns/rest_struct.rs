//! feature: `..` in a struct pattern
//! section: Rest patterns
//! input: 1 2 3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
struct S { a: i16, b: i16, c: i16 }
pub fn main() {
    let s = S { a: inp(), b: inp(), c: inp() };
    let S { b, .. } = s;
    out(b);
    out(s.a + s.c);
}
