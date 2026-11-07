//! feature: A generic `impl<T>` block
//! section: Implementations
//! disposition: easy
//! input: 4 9
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[derive(Clone, Copy)]
struct Pair<T> { a: T, b: T }
impl<T: PartialOrd + Copy> Pair<T> { fn max(&self) -> T { if self.a > self.b { self.a } else { self.b } } }
pub fn main() {
    let p = Pair { a: inp(), b: inp() };
    out(p.max());
}
