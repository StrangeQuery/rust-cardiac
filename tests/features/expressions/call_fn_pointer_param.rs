//! feature: A function pointer passed to a function
//! section: Call expressions
//! disposition: demand
//! note: A call through a pointer is an indirect jump, which CARDIAC can build like an indexed read.
//! input: 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
fn inc(x: i16) -> i16 { x + 1 }
fn dbl(x: i16) -> i16 { x * 2 }
#[inline(never)]
fn twice(f: fn(i16) -> i16, x: i16) -> i16 { f(f(x)) }
pub fn main() {
    let a = inp();
    out(twice(inc, a));
    out(twice(dbl, a));
}
