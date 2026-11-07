//! feature: An inherent method taking `&self`
//! section: Method-call expressions
//! input: 3 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
struct P { x: i16, y: i16 }
impl P {
    #[inline(never)]
    fn sum(&self) -> i16 { self.x + self.y }
}
pub fn main() {
    let p = P { x: inp(), y: inp() };
    out(p.sum());
}
