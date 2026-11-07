//! feature: Operator overloading: `core::ops::Add` on a struct
//! section: Traits
//! input: 1 2 3 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[derive(Clone, Copy)]
struct V { x: i16, y: i16 }
impl core::ops::Add for V { type Output = V; fn add(self, o: V) -> V { V { x: self.x + o.x, y: self.y + o.y } } }
pub fn main() {
    let a = V { x: inp(), y: inp() };
    let b = V { x: inp(), y: inp() };
    let c = a + b;
    out(c.x);
    out(c.y);
}
