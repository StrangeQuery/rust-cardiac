//! feature: Struct update syntax (`S { x: 1, ..s }`)
//! section: Structs
//! input: 3 4 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[derive(Clone, Copy)]
struct S { a: i16, b: i16, c: i16 }
pub fn main() {
    let s = S { a: inp(), b: inp(), c: inp() };
    let t = S { b: 0, ..s };
    out(t.a + t.b + t.c);
}
