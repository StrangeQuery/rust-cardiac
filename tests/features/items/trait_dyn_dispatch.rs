//! feature: Dynamic dispatch: `&dyn Trait` chosen at run time
//! section: Traits
//! disposition: demand
//! note: A `&dyn` is a pointer and a table of functions: a call through it is a computed jump, which self-modifying code can do.
//! input: 1 5 0 5
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
#[inline(never)]
fn apply(o: &dyn Op, x: i16) -> i16 { o.run(x) }
pub fn main() {
    for _ in 0..2 {
        let o: &dyn Op = if inp() == 1 { &Inc } else { &Neg };
        out(apply(o, inp()));
    }
}
