//! feature: Chained methods returning `Self`
//! section: Implementations
//! input: 2
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[derive(Clone, Copy)]
struct B { v: i16 }
impl B {
    fn add(self, k: i16) -> Self { B { v: self.v + k } }
    fn neg(self) -> Self { B { v: -self.v } }
}
pub fn main() {
    out(B { v: inp() }.add(5).neg().add(1).v);
}
