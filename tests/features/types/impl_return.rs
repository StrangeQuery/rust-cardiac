//! feature: `impl Fn(i16) -> i16` returned from a function
//! section: Impl trait type
//! disposition: demand
//! note: At opt-level 1 rustc calls `adder`, which returns a closure: a struct the compiler would have to pass back.
//! input: 3 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
fn adder(k: i16) -> impl Fn(i16) -> i16 {
    move |x| x + k
}
pub fn main() {
    let f = adder(inp());
    out(f(inp()));
}
