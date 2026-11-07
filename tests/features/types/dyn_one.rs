//! feature: A `&dyn Trait` with one implementation
//! section: Trait object types
//! disposition: demand
//! note: Dynamic dispatch needs a vtable and an indirect call, the same machinery as function pointers.
//! input: 7
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
fn show(s: &dyn Shape) {
    out(s.area());
}
pub fn main() {
    show(&Sq(inp()));
}
