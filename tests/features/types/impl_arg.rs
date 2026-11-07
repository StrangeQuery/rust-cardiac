//! feature: `impl Trait` as a parameter type
//! section: Impl trait type
//! disposition: demand
//! note: `impl Trait` in argument position is a generic function.
//! input: 3 4 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
trait Shape {
    fn area(&self) -> i16;
}
struct Sq(i16);
struct Rect(i16, i16);
impl Shape for Sq {
    fn area(&self) -> i16 { self.0 * self.0 }
}
impl Shape for Rect {
    fn area(&self) -> i16 { self.0 * self.1 }
}
#[inline(never)]
fn show(s: impl Shape) {
    out(s.area());
}
pub fn main() {
    show(Rect(inp(), inp()));
    show(Sq(inp()));
}
