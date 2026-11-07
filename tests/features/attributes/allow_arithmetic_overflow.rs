//! feature: `#[allow(arithmetic_overflow)]` over a sum rustc knows overflows
//! section: Diagnostic attributes
//! note: rustc folds the sum; with checks on both builds panic, with checks off both wrap as `i16` does.
//! input: 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[allow(arithmetic_overflow)]
pub fn main() {
    let a = inp();
    out(a);
    let b: i16 = 32767 + 1;
    out(b - 32000 + a);
}
