//! feature: `#[derive(Clone, Copy)]` and `.clone()`
//! section: Traits
//! input: 9
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[derive(Clone, Copy)]
struct P { x: i16 }
pub fn main() {
    let a = P { x: inp() };
    let mut b = a.clone();
    b.x += 1;
    out(a.x);
    out(b.x);
}
