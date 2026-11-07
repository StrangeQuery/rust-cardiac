//! feature: `a + b` past a card (999 + 1)
//! section: Arithmetic operators
//! disposition: never
//! input: 999 1
//! differs: A cell holds -999 to 999. With overflow checks on, CARDIAC panics where native Rust prints 1000; with them off it keeps the low three digits.
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    let b = inp();
    out(a + b);
}
