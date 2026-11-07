//! feature: Calling through a function pointer chosen at run time
//! section: Call expressions
//! disposition: demand
//! note: A call through a pointer is an indirect jump, which CARDIAC can build like an indexed read.
//! input: 1 5 0 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
fn inc(x: i16) -> i16 { x + 1 }
fn dec(x: i16) -> i16 { x - 1 }
pub fn main() {
    for _ in 0..2 {
        let c = inp();
        let f: fn(i16) -> i16 = if c != 0 { inc } else { dec };
        out(f(inp()));
    }
}
