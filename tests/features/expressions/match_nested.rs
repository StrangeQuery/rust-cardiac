//! feature: `match` on a nested pattern, `Some(Shape::Sq(a))`
//! section: Match expressions
//! input: 1 3 2 0 0 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[derive(Clone, Copy)]
enum Shape { Dot, Sq(i16) }
#[inline(never)]
fn mk(k: i16, x: i16) -> Option<Shape> {
    if k == 1 { Some(Shape::Sq(x)) } else if k == 2 { Some(Shape::Dot) } else { None }
}
pub fn main() {
    for _ in 0..3 {
        out(match mk(inp(), inp()) { Some(Shape::Sq(a)) => a, Some(Shape::Dot) => 0, None => -1 });
    }
}
