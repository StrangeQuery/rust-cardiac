//! feature: An array of structs, `t[i].f`
//! section: Array and array index expressions
//! input: 1 9
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
    let mut t = [P { x: 0, y: 0 }; 3];
    let i = inp() as usize;
    t[i] = mk(inp(), 4);
    out(t[i].x + t[i].y);
    out(t[0].x);
}
