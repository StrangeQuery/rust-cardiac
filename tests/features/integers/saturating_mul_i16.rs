//! feature: `a.saturating_mul(b)` on `i16`, on a card
//! section: Saturating
//! disposition: demand
//! note: As for `saturating_add`. At opt-level 2 the compiler crashes: rustc's inlined version picks between 32767 and -32768, and the compiler panics on a constant no card holds instead of refusing it.
//! input: 30 -30
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    let b = inp();
    out(a.saturating_mul(b));
}
