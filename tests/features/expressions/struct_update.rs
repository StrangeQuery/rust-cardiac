//! feature: Functional update, `P { x: 1, ..base }`
//! section: Struct expressions
//! input: 3 4 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[derive(Clone, Copy)]
struct P { x: i16, y: i16, z: i16 }
#[inline(never)]
fn show(p: P) { out(p.x); out(p.y); out(p.z); }
pub fn main() {
    let base = P { x: inp(), y: inp(), z: inp() };
    let q = P { y: 0, ..base };
    show(q);
    show(base);
}
