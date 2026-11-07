//! feature: A closure passed as `&dyn Fn`
//! section: Closure expressions
//! disposition: demand
//! note: Dynamic dispatch needs a vtable and an indirect call, which CARDIAC can build as it builds an indexed read.
//! input: 3 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn apply(f: &dyn Fn(i16) -> i16, x: i16) -> i16 { f(f(x)) }
pub fn main() {
    let k = inp();
    out(apply(&|a| a + k, inp()));
}
