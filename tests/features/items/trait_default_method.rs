//! feature: A trait's default method
//! section: Traits
//! disposition: easy
//! input: 6
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
trait Shape {
    fn side(&self) -> i16;
    fn perimeter(&self) -> i16 { self.side() * 4 }
}
struct Sq(i16);
impl Shape for Sq { fn side(&self) -> i16 { self.0 } }
pub fn main() {
    out(Sq(inp()).perimeter());
}
