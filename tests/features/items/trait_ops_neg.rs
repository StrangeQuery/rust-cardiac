//! feature: `Neg` on a struct
//! section: Traits
//! input: 3 -5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[derive(Clone, Copy)]
struct V { x: i16, y: i16 }
impl core::ops::Neg for V { type Output = V; fn neg(self) -> V { V { x: -self.x, y: -self.y } } }
pub fn main() {
    let a = -V { x: inp(), y: inp() };
    out(a.x);
    out(a.y);
}
