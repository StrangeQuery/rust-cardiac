//! feature: Direct recursion (`fn sum(n) { n + sum(n - 1) }`)
//! section: Functions
//! disposition: demand
//! note: CARDIAC has no stack and each function's locals and return address are fixed cells, so a recursive call needs a software stack of its own (an array and self-modifying loads); possible, but slow and costly in cells, and a loop does the same.
//! input: 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn sum(n: i16) -> i16 { if n <= 0 { 0 } else { n + sum(n - 1) } }
pub fn main() {
    out(sum(inp()));
}
