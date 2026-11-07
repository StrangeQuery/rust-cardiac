//! feature: `a.count_ones()` on `i16`
//! section: Bits
//! disposition: demand
//! note: Bit counts are `u32`s, and on a decimal machine a loop of halvings.
//! input: 7 8 255 -1 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    loop {
        let a = inp();
        out(a.count_ones() as i16);
        if a == 0 {
            break;
        }
    }
}
