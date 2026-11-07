//! feature: `a.is_power_of_two()` on `usize`
//! section: Powers and roots
//! disposition: demand
//! note: As for `u8`.
//! input: 1 3 4 512 999 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    loop {
        let a = inp() as usize;
        out(a.is_power_of_two() as i16);
        if a == 0 {
            break;
        }
    }
}
