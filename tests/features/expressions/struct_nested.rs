//! feature: A struct literal with a struct field
//! section: Struct expressions
//! input: 3 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
struct P { x: i16, y: i16 }
struct Line { a: P, b: P }
#[inline(never)]
fn len(l: &Line) -> i16 { (l.b.x - l.a.x) + (l.b.y - l.a.y) }
pub fn main() {
    let x = inp();
    let y = inp();
    let l = Line { a: P { x: 0, y: 0 }, b: P { x, y } };
    out(len(&l));
}
