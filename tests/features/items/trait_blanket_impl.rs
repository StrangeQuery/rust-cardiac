//! feature: A blanket impl (`impl<T: A> B for T`)
//! section: Traits
//! disposition: easy
//! input: 6
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
trait Base { fn get(&self) -> i16; }
trait Doubled { fn doubled(&self) -> i16; }
impl<T: Base> Doubled for T { fn doubled(&self) -> i16 { self.get() * 2 } }
struct X(i16);
impl Base for X { fn get(&self) -> i16 { self.0 } }
pub fn main() {
    out(X(inp()).doubled());
}
