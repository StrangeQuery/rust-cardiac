//! feature: An enum whose variants carry data, built from `inp()` in a function that returns it
//! section: Enumerations
//! note: A wrong answer: `Shape::Rect(inp(), inp())` returned from a function loses its second field (prints 3 for 3 - 4); with the fields read into locals first it is right. Even `fn read() -> Shape { Shape::Rect(inp(), inp()) }` alone gives 0, 0.
//! input: 0 5 1 3 4 2
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
enum Shape { Sq(i16), Rect(i16, i16), Dot }
#[inline(never)]
fn read() -> Shape {
    match inp() { 0 => Shape::Sq(inp()), 1 => Shape::Rect(inp(), inp()), _ => Shape::Dot }
}
#[inline(never)]
fn area(s: Shape) -> i16 { match s { Shape::Sq(a) => a + a, Shape::Rect(a, b) => a - b, Shape::Dot => 0 } }
pub fn main() {
    for _ in 0..3 { out(area(read())); }
}
