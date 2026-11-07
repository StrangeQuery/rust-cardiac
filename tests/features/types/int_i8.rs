//! feature: `i8`: declared, added, subtracted, compared and cast back to `i16`
//! section: Integer types
//! disposition: easy
//! note: An `i8` holds -128..=127, all on a card; only the overflow check at 127 is new.
//! input: 50 -20
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a: i8 = inp() as i8;
    let b: i8 = inp() as i8;
    out((a + b) as i16);
    out((a - b) as i16);
    out((a < b) as i16);
    out((a == b) as i16);
}
