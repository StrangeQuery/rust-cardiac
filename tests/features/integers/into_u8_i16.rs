//! feature: `a.into()` from `u8` to `i16`
//! section: Conversions
//! disposition: easy
//! note: As for `i16::from`.
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
        let b: i16 = U[i as usize].into();
        out(b);
    }
}
