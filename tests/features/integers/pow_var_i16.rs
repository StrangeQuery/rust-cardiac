//! feature: `a.pow(n)` on `i16`, `n` from a card
//! section: Powers and roots
//! disposition: demand
//! note: The exponent is a `u32`, which the compiler lacks; after that it is a loop of multiplies.
//! input: 2 3 -3 3 10 2 5 0 0 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    loop {
        let a = inp();
        let b = inp();
        if a == 0 && b == 0 {
            break;
        }
        out(a.pow(b as u32));
    }
}
