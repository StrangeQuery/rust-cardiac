//! feature: `let Point { x, y } = p;`
//! section: Destructuring
//! input: 4 9 -8 2
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
struct Point { x: i16, y: i16 }
#[inline(never)]
fn make(x: i16, y: i16) -> Point { Point { x, y } }
pub fn main() {
    for _ in 0..2 {
        let p = make(inp(), inp());
        let Point { x, y } = p;
        out(y - x);
    }
}
