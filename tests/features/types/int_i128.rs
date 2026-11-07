//! feature: `i128`: declared, added, subtracted, compared and cast back to `i16`
//! section: Integer types
//! disposition: never
//! input: 400 -300
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a: i128 = inp() as i128;
    let b: i128 = inp() as i128;
    out((a + b) as i16);
    out((a - b) as i16);
    out((a < b) as i16);
    out((a == b) as i16);
}
