//! feature: `#[derive(Clone, Copy)]` on an enum with data: `.clone()`
//! section: Derive
//! input: 7 -8 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[derive(Clone, Copy)]
enum Shape { Dot, Circle(i16) }
fn size(s: Shape) -> i16 {
    match s { Shape::Dot => 0, Shape::Circle(r) => r }
}
pub fn main() {
    loop {
        let v = inp();
        if v == 0 { break; }
        let s = if v > 0 { Shape::Circle(v) } else { Shape::Dot };
        let t = s.clone();
        out(size(s) + size(t));
    }
}
