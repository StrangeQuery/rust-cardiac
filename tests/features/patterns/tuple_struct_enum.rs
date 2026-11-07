//! feature: Tuple variant patterns (`Shape::Rect(w, h)`)
//! section: Tuple struct patterns
//! note: A wrong answer, and not from the pattern: an enum value built in an `#[inline(never)]` function from `inp()` can lose a field (`fn make() -> Option<i16> { Some(inp()) }` gives `Some(0)` for a card of 4); built from parameters it is right.
//! input: 1 3 2 4 5 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
enum Shape { Dot, Circle(i16), Rect(i16, i16) }
#[inline(never)]
fn make(k: i16) -> Shape {
    match k { 1 => Shape::Circle(inp()), 2 => Shape::Rect(inp(), inp()), _ => Shape::Dot }
}
pub fn main() {
    loop {
        let k = inp();
        if k == 0 { break; }
        let r = match make(k) {
            Shape::Dot => 0,
            Shape::Circle(r) => r * 3,
            Shape::Rect(w, h) => w + h,
        };
        out(r);
    }
}
