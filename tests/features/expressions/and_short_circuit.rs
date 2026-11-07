//! feature: `&&` doesn't evaluate its right side when the left side is false
//! section: Lazy boolean operators
//! note: The right side divides by zero: evaluating it would panic.
//! input: 0 7 2 7
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    for _ in 0..2 {
        let d = inp();
        let a = inp();
        out((d != 0 && a / d > 1) as i16);
    }
}
