//! feature: `#[derive(Clone, Copy)]` on a struct: `.clone()` and copies
//! section: Derive
//! input: 3 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[derive(Clone, Copy)]
struct P { x: i16, y: i16 }
pub fn main() {
    let a = P { x: inp(), y: inp() };
    let mut b = a.clone();
    let c = a;
    b.x += 10;
    out(a.x); out(b.x); out(c.y);
}
