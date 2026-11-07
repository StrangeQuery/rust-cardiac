//! feature: `isize`: declared, added, subtracted, compared and cast back to `i16`
//! section: Machine-dependent integer types
//! disposition: demand
//! note: An `isize` is 64 bits natively, but one that fits on a card can be held as an `i16` is.
//! input: 400 -300
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a: isize = inp() as isize;
    let b: isize = inp() as isize;
    out((a + b) as i16);
    out((a - b) as i16);
    out((a < b) as i16);
    out((a == b) as i16);
}
