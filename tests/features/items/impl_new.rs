//! feature: An associated function `new()` and `Self`
//! section: Implementations
//! input: 2 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
struct R { lo: i16, hi: i16 }
impl R {
    #[inline(never)]
    fn new(a: i16, b: i16) -> Self { if a < b { Self { lo: a, hi: b } } else { Self { lo: b, hi: a } } }
    fn width(&self) -> i16 { self.hi - self.lo }
}
pub fn main() {
    let r = R::new(inp(), inp());
    out(r.width());
}
