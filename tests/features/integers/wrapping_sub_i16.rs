//! feature: `a.wrapping_sub(b)` on `i16`, on a card
//! section: Wrapping
//! input: 7 3 -7 3 500 -499 0 0
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
        out(a.wrapping_sub(b));
    }
}
