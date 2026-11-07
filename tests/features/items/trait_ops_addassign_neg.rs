//! feature: `AddAssign` on a struct
//! section: Traits
//! input: 3 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[derive(Clone, Copy)]
struct V { x: i16, y: i16 }
impl core::ops::AddAssign for V { fn add_assign(&mut self, o: V) { self.x += o.x; self.y += o.y; } }
pub fn main() {
    let mut a = V { x: inp(), y: inp() };
    let b = a;
    a += b;
    out(a.x);
    out(a.y);
}
