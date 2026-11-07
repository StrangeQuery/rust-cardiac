//! feature: An enum variant built from a call's result, `Shape::Rect(k, inp())`, and returned
//! section: Struct expressions
//! note: Wrong answer: the field set from the call's result comes back as 0; built from parameters the same enum is right.
//! input: 1 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
enum Shape { Dot, Rect(i16, i16) }
#[inline(never)]
fn mk(k: i16) -> Shape {
    if k == 0 { Shape::Dot } else { Shape::Rect(k, inp()) }
}
pub fn main() {
    let s = mk(inp());
    out(match s { Shape::Dot => 0, Shape::Rect(a, b) => a + b * 10 });
}
