//! feature: A closure that captures nothing, coerced to a `fn` pointer
//! section: Closure expressions
//! disposition: demand
//! note: A call through a pointer is an indirect jump, which CARDIAC can build like an indexed read.
//! input: 1 5 0 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    for _ in 0..2 {
        let c = inp();
        let f: fn(i16) -> i16 = if c != 0 { |x| x + 1 } else { |x| x - 1 };
        out(f(inp()));
    }
}
