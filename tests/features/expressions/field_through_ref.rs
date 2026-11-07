//! feature: A field read through `&mut` without `*`, `r.f`
//! section: Field access expressions
//! input: 3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
struct P { x: i16, y: i16 }
#[inline(never)]
fn grow(p: &mut P) { p.x += p.y; p.y += 1; }
pub fn main() {
    let mut p = P { x: inp(), y: 5 };
    grow(&mut p);
    grow(&mut p);
    out(p.x);
    out(p.y);
}
