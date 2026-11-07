//! feature: Chained methods, `p.moved(1).moved(2).sum()`
//! section: Method-call expressions
//! input: 3 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[derive(Clone, Copy)]
struct P { x: i16, y: i16 }
impl P {
    #[inline(never)]
    fn moved(self, d: i16) -> P { P { x: self.x + d, y: self.y - d } }
    #[inline(never)]
    fn sum(self) -> i16 { self.x * 10 + self.y }
}
pub fn main() {
    let p = P { x: inp(), y: inp() };
    out(p.moved(1).moved(2).sum());
}
