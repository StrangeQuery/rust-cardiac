//! feature: A closure returned from a function (`impl Fn`)
//! section: Closure expressions
//! disposition: demand
//! note: Each closure is its own type, so the function returning one is generic, compiled once per closure.
//! input: 3 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn adder(k: i16) -> impl Fn(i16) -> i16 { move |a| a + k }
pub fn main() {
    let f = adder(inp());
    out(f(inp()));
    out(f(10));
}
