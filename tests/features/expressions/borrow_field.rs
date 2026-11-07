//! feature: `&mut s.f` to a struct field, passed to a function
//! section: Borrow operators
//! input: 7
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
struct P { x: i16, y: i16 }
#[inline(never)]
fn bump(v: &mut i16) { *v += 1; }
pub fn main() {
    let mut p = P { x: inp(), y: 50 };
    bump(&mut p.x);
    bump(&mut p.y);
    out(p.x);
    out(p.y);
}
