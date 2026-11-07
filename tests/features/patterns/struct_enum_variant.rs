//! feature: Struct-like enum variant (`Shape::Rect { w, h }`)
//! section: Struct patterns
//! disposition: easy
//! input: 1 3 4 2 5 0 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
enum Shape { Square { s: i16 }, Rect { w: i16, h: i16 } }
#[inline(never)]
fn make(k: i16, a: i16) -> Shape {
    if k == 1 { Shape::Square { s: a } } else { Shape::Rect { w: a, h: inp() } }
}
pub fn main() {
    loop {
        let k = inp();
        if k == 0 { break; }
        let a = inp();
        let r = match make(k, a) {
            Shape::Square { s } => s + s,
            Shape::Rect { w, h } => w + h,
        };
        out(r);
    }
}
