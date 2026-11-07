//! feature: A generic struct at two types
//! section: Implementations
//! disposition: easy
//! note: `Pair<i16>` and `Pair<bool>` are two structs, each with its own cells.
//! input: 4 9 1 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[derive(Clone, Copy)]
struct Pair<T> { a: T, b: T }
impl<T: Copy> Pair<T> { #[inline(never)] fn second(&self) -> T { self.b } }
pub fn main() { let p = Pair { a: inp(), b: inp() }; out(p.second()); let q = Pair { a: inp() > 0, b: inp() > 0 }; out(q.second() as i16); }
