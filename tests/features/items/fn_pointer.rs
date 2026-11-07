//! feature: A function pointer (`fn(i16) -> i16`) chosen at run time
//! section: Functions
//! disposition: demand
//! note: A call through a cell holding an address can be built with self-modifying code, as variable indexing is.
//! input: 1 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
fn dbl(x: i16) -> i16 { x + x }
fn neg(x: i16) -> i16 { -x }
pub fn main() {
    let f: fn(i16) -> i16 = if inp() == 1 { dbl } else { neg };
    out(f(inp()));
}
