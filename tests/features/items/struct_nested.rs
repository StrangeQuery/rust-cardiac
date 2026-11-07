//! feature: A struct inside a struct
//! section: Structs
//! input: 1 2 5 7
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[derive(Clone, Copy)]
struct P { x: i16, y: i16 }
#[derive(Clone, Copy)]
struct Line { from: P, to: P }
#[inline(never)]
fn len(l: Line) -> i16 { (l.to.x - l.from.x) + (l.to.y - l.from.y) }
pub fn main() {
    let l = Line { from: P { x: inp(), y: inp() }, to: P { x: inp(), y: inp() } };
    out(len(l));
    out(l.to.x);
}
