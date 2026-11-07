//! feature: `a.leading_zeros()` on `i16`
//! section: Bits
//! disposition: demand
//! note: As for `count_ones`.
//! input: 1 7 256 -1 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    loop {
        let a = inp();
        out(a.leading_zeros() as i16);
        if a == 0 {
            break;
        }
    }
}
