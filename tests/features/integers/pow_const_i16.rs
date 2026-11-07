//! feature: `a.pow(3)` on `i16`
//! section: Powers and roots
//! disposition: easy
//! note: With a constant exponent `pow` is a few multiplies, but rustc calls it at both opt-levels.
//! input: 2 -3 9 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    loop {
        let a = inp();
        out(a.pow(3));
        if a == 0 {
            break;
        }
    }
}
