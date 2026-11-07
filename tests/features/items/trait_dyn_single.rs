//! feature: A `&dyn Trait` with one implementation
//! section: Traits
//! disposition: demand
//! note: A `&dyn` is a pointer and a table of functions: a call through it is a computed jump, which self-modifying code can do.
//! input: 8
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
trait Val { fn val(&self) -> i16; }
struct A(i16);
impl Val for A { fn val(&self) -> i16 { self.0 + 1 } }
#[inline(never)]
fn show(v: &dyn Val) { out(v.val()); }
pub fn main() {
    show(&A(inp()));
}
