//! feature: `i16::from(a)` for a `u8`
//! section: Conversions
//! disposition: easy
//! note: At opt-level 1 rustc calls `From::from`; it is `as i16`, which works.
//! input: 0 1 2 3 4 5 6 7 -1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
const U: [u8; 8] = [0, 1, 2, 7, 100, 200, 254, 255];
pub fn main() {
    loop {
        let i = inp();
        if i < 0 {
            break;
        }
        let a = U[i as usize];
        out(i16::from(a));
    }
}
