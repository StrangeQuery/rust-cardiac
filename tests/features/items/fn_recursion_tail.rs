//! feature: Tail recursion (`fn count(n, acc)`)
//! section: Functions
//! disposition: easy
//! note: A call in tail position needs no stack: the compiler could turn it into a jump back to the start, as a loop.
//! input: 6
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn count(n: i16, acc: i16) -> i16 { if n == 0 { acc } else { count(n - 1, acc + 2) } }
pub fn main() {
    out(count(inp(), 0));
}
