//! feature: `a.wrapping_div(b)` on `i16`, on a card
//! section: Wrapping
//! disposition: easy
//! note: On a card it is plain `/`; rustc calls `overflowing_div` even at opt-level 2, for the `i16::MIN / -1` case that no card holds.
//! input: 7 3 -7 3 999 -9 0 0
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
        out(a.wrapping_div(b));
    }
}
