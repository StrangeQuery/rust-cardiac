//! feature: `.hash()` of a `#[derive(Hash)]` value with a hand-written `Hasher`
//! section: Derive
//! disposition: demand
//! note: At opt-level 1 the struct is passed whole to the generic derived `hash`.
//! note: The core library's `Hasher` works in `u64`, and `write_i16` goes through `to_ne_bytes`; a hand-written `Hasher` that folds `i16`s stays on cards.
//! input: 3 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
use core::hash::{Hash, Hasher};
#[derive(Hash)]
struct P { x: i16, y: i16 }
struct Sum(i16);
impl Hasher for Sum {
    fn finish(&self) -> u64 { 0 }
    fn write(&mut self, _b: &[u8]) {}
    fn write_i16(&mut self, v: i16) { self.0 = self.0 + self.0 + v; }
}
pub fn main() {
    let p = P { x: inp(), y: inp() };
    let mut h = Sum(0);
    p.hash(&mut h);
    out(h.0);
}
