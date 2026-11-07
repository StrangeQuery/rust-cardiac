//! feature: `i16 as u8` (it wraps)
//! section: Type cast expressions
//! disposition: easy
//! note: Wrapping is `a.rem_euclid(256)` on a card.
//! input: 5 255 300 -1 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    loop {
        let a = inp();
        let b = a as u8;
        out(b as i16);
        if a == 0 {
            break;
        }
    }
}
