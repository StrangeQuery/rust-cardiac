//! feature: `a / 10`: by a power of ten
//! section: Arithmetic operators
//! note: A shift. Rounds toward zero: `-7 / 10` is 0.
//! input: 7 -7 99 -99 123 -123 999 -999 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    loop {
        let a = inp();
        out(a / 10);
        if a == 0 {
            break;
        }
    }
}
