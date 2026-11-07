//! feature: A unit struct and its method
//! section: Struct expressions
//! disposition: easy
//! note: A unit struct has no fields, so it needs no cells at all.
//! input: 3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
struct Doubler;
impl Doubler {
    #[inline(never)]
    fn apply(&self, x: i16) -> i16 { x + x }
}
pub fn main() {
    let d = Doubler;
    out(d.apply(inp()));
}
