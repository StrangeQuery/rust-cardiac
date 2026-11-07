//! feature: A trait method, static dispatch
//! section: Traits
//! input: 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
trait Twice { fn twice(self) -> Self; }
impl Twice for i16 { fn twice(self) -> i16 { self + self } }
pub fn main() {
    out(inp().twice());
}
