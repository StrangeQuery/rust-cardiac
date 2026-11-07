//! feature: An enum whose variants carry data, built from locals and passed to a function
//! section: Enumerations
//! input: 3 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
enum Shape { Sq(i16), Rect(i16, i16) }
#[inline(never)]
fn area(s: Shape) -> i16 { match s { Shape::Sq(a) => a + a, Shape::Rect(a, b) => a * b } }
pub fn main() {
    let a = inp();
    let b = inp();
    out(area(Shape::Rect(a, b)));
    out(area(Shape::Sq(a)));
}
