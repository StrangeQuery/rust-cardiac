//! feature: A method of a generic `impl<T>` block that stays a call
//! section: Implementations
//! disposition: easy
//! note: `#[inline(never)]` keeps the method a call at every opt-level, so `Pair<i16>` is a local of its own: each instance of a generic struct is laid out as a plain struct.
//! input: 4 9
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[derive(Clone, Copy)]
struct Pair<T> { a: T, b: T }
impl<T: PartialOrd + Copy> Pair<T> { #[inline(never)] fn max(&self) -> T { if self.a > self.b { self.a } else { self.b } } }
pub fn main() { let p = Pair { a: inp(), b: inp() }; out(p.max()); }
