//! feature: Nested field access, `a.b.c`, read and written
//! section: Field access expressions
//! input: 3 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
struct In { c: i16, d: i16 }
struct Out { b: In, e: i16 }
pub fn main() {
    let mut a = Out { b: In { c: inp(), d: inp() }, e: 1 };
    a.b.c += a.e;
    a.b.d = a.b.c * 2;
    out(a.b.c);
    out(a.b.d);
}
