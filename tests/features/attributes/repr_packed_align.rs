//! feature: `#[repr(packed)]` and `#[repr(align(4))]` on structs
//! section: Type layout (repr)
//! input: 3 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[repr(C, packed)]
#[derive(Clone, Copy)]
struct P { a: u8, b: i16 }
#[repr(align(4))]
#[derive(Clone, Copy)]
struct Q { x: i16 }
#[inline(never)]
fn sum(p: P, q: Q) -> i16 { p.a as i16 + p.b + q.x }
pub fn main() {
    let k = inp();
    let a: u8 = if k > 2 { 3 } else { 1 };
    let p = P { a, b: inp() };
    out(sum(p, Q { x: 10 }));
}
