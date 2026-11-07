//! feature: `u8`: declared, added, subtracted, compared and cast back to `i16`
//! section: Integer types
//! input: 4 3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
const U: [u8; 6] = [0, 3, 5, 50, 200, 255];
pub fn main() {
    let a: u8 = U[inp() as usize];
    let b: u8 = U[inp() as usize];
    out((a + b) as i16);
    out((a - b) as i16);
    out((a < b) as i16);
    out((a == b) as i16);
}
