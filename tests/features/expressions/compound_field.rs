//! feature: `s.f += a` on a struct field
//! section: Assignment and compound assignment
//! input: 5 -3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
struct P {
    x: i16,
    y: i16,
}
pub fn main() {
    let mut p = P { x: 1, y: 2 };
    p.x += inp();
    p.y -= inp();
    out(p.x);
    out(p.y);
}
