//! feature: An array of `i8`
//! section: Array types
//! disposition: easy
//! input: 5 -6 7 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let mut t: [i8; 3] = [0; 3];
    let mut k = 0;
    while k < 3 {
        t[k] = inp() as i8;
        k += 1;
    }
    let i = inp() as usize;
    let x = t[i];
    let y = t[2 - i];
    out(x as i16);
    out(y as i16);
}
