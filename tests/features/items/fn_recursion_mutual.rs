//! feature: Mutual recursion (`is_even` and `is_odd`)
//! section: Functions
//! disposition: demand
//! note: As for direct recursion: it needs a software stack, since each function's cells are fixed.
//! input: 7 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn is_even(n: i16) -> bool { if n == 0 { true } else { is_odd(n - 1) } }
#[inline(never)]
fn is_odd(n: i16) -> bool { if n == 0 { false } else { is_even(n - 1) } }
pub fn main() {
    out(is_even(inp()) as i16);
    out(is_even(inp()) as i16);
}
