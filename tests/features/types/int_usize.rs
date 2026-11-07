//! feature: `usize`: declared, added, subtracted, compared and cast back to `i16`
//! section: Machine-dependent integer types
//! input: 400 300
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a: usize = inp() as usize;
    let b: usize = inp() as usize;
    out((a + b) as i16);
    out((a - b) as i16);
    out((a < b) as i16);
    out((a == b) as i16);
}
