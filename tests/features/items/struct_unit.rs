//! feature: A unit struct with a method
//! section: Structs
//! disposition: easy
//! note: A unit struct holds nothing, so it needs no cells.
//! input: 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
struct Doubler;
impl Doubler { fn apply(&self, x: i16) -> i16 { x + x } }
pub fn main() {
    let d = Doubler;
    out(d.apply(inp()));
}
