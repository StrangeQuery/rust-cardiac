//! feature: An array of `u8`
//! section: Array types
//! input: 1 4 5 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
const U: [u8; 6] = [0, 3, 5, 50, 200, 255];
pub fn main() {
    let mut t: [u8; 3] = [0; 3];
    let mut k = 0;
    while k < 3 {
        t[k] = U[inp() as usize];
        k += 1;
    }
    let i = inp() as usize;
    let x = t[i];
    let y = t[2 - i];
    out(x as i16);
    out(y as i16);
}
