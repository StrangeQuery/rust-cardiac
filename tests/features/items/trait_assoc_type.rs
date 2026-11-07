//! feature: An associated type in a trait
//! section: Traits
//! disposition: easy
//! input: 7
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
trait Make { type Out; fn make(&self, v: i16) -> Self::Out; }
struct Flag;
impl Make for Flag { type Out = bool; fn make(&self, v: i16) -> bool { v > 5 } }
pub fn main() {
    let x: bool = Flag.make(inp());
    out(x as i16);
}
