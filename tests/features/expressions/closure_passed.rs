//! feature: A closure passed to a generic function
//! section: Closure expressions
//! disposition: demand
//! note: Each closure is its own type, so the function it's passed to is generic, compiled once per closure.
//! input: 3 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn apply<F: Fn(i16) -> i16>(f: F, x: i16) -> i16 { f(f(x)) }
pub fn main() {
    let k = inp();
    out(apply(|a| a + k, inp()));
}
