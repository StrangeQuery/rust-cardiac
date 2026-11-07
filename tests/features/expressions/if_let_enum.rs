//! feature: `if let` on an enum variant with fields
//! section: If and if let expressions
//! input: 1 4 5 2 4 5 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
enum Shape { Dot, Rect(i16, i16) }
#[inline(never)]
fn mk(k: i16, a: i16, b: i16) -> Shape { if k == 1 { Shape::Rect(a, b) } else { Shape::Dot } }
pub fn main() {
    loop {
        let k = inp();
        if k == 0 { break; }
        let s = mk(k, inp(), inp());
        if let Shape::Rect(w, h) = s { out(w + h); } else { out(0); }
    }
}
