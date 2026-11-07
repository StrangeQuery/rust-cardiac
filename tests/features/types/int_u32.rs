//! feature: `u32`: declared, added, subtracted, compared and cast back to `i16`
//! section: Integer types
//! disposition: never
//! input: 400 300
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a: u32 = inp() as u32;
    let b: u32 = inp() as u32;
    out((a + b) as i16);
    out((a - b) as i16);
    out((a < b) as i16);
    out((a == b) as i16);
}
