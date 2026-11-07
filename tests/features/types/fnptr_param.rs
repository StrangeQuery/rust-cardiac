//! feature: A function pointer passed as a parameter
//! section: Function pointer types
//! disposition: demand
//! input: 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn double(x: i16) -> i16 { x * 2 }
#[inline(never)]
fn apply(f: fn(i16) -> i16, x: i16) -> i16 { f(x) + 1 }
pub fn main() {
    out(apply(double, inp()));
}
