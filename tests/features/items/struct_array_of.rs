//! feature: An array of structs
//! section: Structs
//! input: 2 3 4 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[derive(Clone, Copy)]
struct P { x: i16, y: i16 }
#[inline(never)]
fn mk(x: i16, y: i16) -> P { P { x, y } }
pub fn main() {
    let mut t = [P { x: 0, y: 0 }; 2];
    for i in 0..2 { t[i] = mk(inp(), inp()); }
    out(t[0].x + t[1].y);
    out(t[1].x - t[0].y);
}
