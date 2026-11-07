//! feature: A function pointer `fn(i16) -> i16` chosen at run time
//! section: Function pointer types
//! disposition: demand
//! note: An indirect call is a computed jump, which CARDIAC can make with self-modifying code.
//! input: 1 5 -1 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn double(x: i16) -> i16 { x * 2 }
#[inline(never)]
fn neg(x: i16) -> i16 { -x }
pub fn main() {
    let mut k = 0;
    while k < 2 {
        let f: fn(i16) -> i16 = if inp() > 0 { double } else { neg };
        out(f(inp()));
        k += 1;
    }
}
