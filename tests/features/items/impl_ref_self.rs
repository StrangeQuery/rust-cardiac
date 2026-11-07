//! feature: A method taking `&self`, not inlined
//! section: Implementations
//! input: 3 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
struct P { x: i16, y: i16 }
impl P { #[inline(never)] fn diff(&self) -> i16 { self.y - self.x } }
pub fn main() {
    let p = P { x: inp(), y: inp() };
    out(p.diff());
    out(p.x);
}
