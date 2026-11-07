//! feature: `match` on an enum with fields
//! section: Match expressions
//! input: 0 0 0 1 3 0 2 3 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
enum Shape { Dot, Sq(i16), Rect(i16, i16) }
#[inline(never)]
fn mk(k: i16, x: i16, y: i16) -> Shape {
    match k { 0 => Shape::Dot, 1 => Shape::Sq(x), _ => Shape::Rect(x, y) }
}
pub fn main() {
    for _ in 0..3 {
        let s = mk(inp(), inp(), inp());
        out(match s { Shape::Dot => 0, Shape::Sq(a) => a + a, Shape::Rect(a, b) => a + b * 10 });
    }
}
