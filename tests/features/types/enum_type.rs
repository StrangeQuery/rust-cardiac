//! feature: An enum type with and without data
//! section: Enumerated types
//! input: 1 5 -1 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
enum Shape {
    Dot,
    Line(i16),
}
#[inline(never)]
fn make(k: i16, v: i16) -> Shape {
    if k > 0 { Shape::Line(v) } else { Shape::Dot }
}
#[inline(never)]
fn size(s: Shape) -> i16 {
    match s {
        Shape::Dot => 0,
        Shape::Line(n) => n,
    }
}
pub fn main() {
    let a = make(inp(), inp());
    let b = make(inp(), inp());
    out(size(a));
    out(size(b));
}
