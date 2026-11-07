//! feature: `a.isqrt()` on `i16`
//! section: Powers and roots
//! disposition: demand
//! note: It works in `u16`, which the compiler lacks; a short loop would do on a card.
//! input: 1 15 16 999 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    loop {
        let a = inp();
        out(a.isqrt());
        if a == 0 {
            break;
        }
    }
}
