//! feature: Auto-ref and auto-deref: a `&self` method called on a value, on `&P` and on `&&P`
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
    let r = &p;
    let rr = &r;
    out(p.sum());
    out(r.sum());
    out(rr.sum());
}
