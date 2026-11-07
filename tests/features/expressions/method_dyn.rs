//! feature: A trait object, `&dyn Trait`, chosen at run time
//! section: Method-call expressions
//! disposition: demand
//! note: Dynamic dispatch needs a vtable and an indirect call, which CARDIAC can build as it builds an indexed read.
//! input: 1 3 0 3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
trait Op { fn run(&self, x: i16) -> i16; }
struct Inc;
struct Neg;
impl Op for Inc { fn run(&self, x: i16) -> i16 { x + 1 } }
impl Op for Neg { fn run(&self, x: i16) -> i16 { -x } }
pub fn main() {
    for _ in 0..2 {
        let c = inp();
        let op: &dyn Op = if c != 0 { &Inc } else { &Neg };
        out(op.run(inp()));
    }
}
