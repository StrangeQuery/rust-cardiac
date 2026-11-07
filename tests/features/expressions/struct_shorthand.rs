//! feature: Field init shorthand, `P { x, y }`
//! section: Struct expressions
//! input: 3 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
struct P { x: i16, y: i16 }
#[inline(never)]
fn mk(x: i16, y: i16) -> P { P { x, y } }
pub fn main() {
    let x = inp();
    let y = inp();
    let p = mk(y, x);
    out(p.x);
    out(p.y);
}
