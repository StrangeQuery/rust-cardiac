//! feature: `u8 as usize` and `usize as u8`
//! section: Integer types
//! input: 4 7
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
const U: [u8; 6] = [0, 3, 5, 50, 200, 255];
pub fn main() {
    let a = U[inp() as usize];
    let b = inp() as usize;
    let c = a as usize + b;
    out(c as i16);
    out((b as u8) as i16);
}
