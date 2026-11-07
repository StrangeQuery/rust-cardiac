//! feature: A supertrait
//! section: Traits
//! disposition: easy
//! input: 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
trait Named { fn id(&self) -> i16; }
trait Shown: Named { fn show(&self) { out(self.id() * 10); } }
struct A(i16);
impl Named for A { fn id(&self) -> i16 { self.0 } }
impl Shown for A {}
pub fn main() {
    A(inp()).show();
}
