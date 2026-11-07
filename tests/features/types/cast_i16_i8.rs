//! feature: `i16 as i8` and `i8 as i16`
//! section: Integer types
//! disposition: easy
//! note: The values here fit in an `i8`, so the cast keeps them as they are.
//! input: 100 -100
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp() as i8;
    let b = inp() as i8;
    out(a as i16);
    out(b as i16);
    out((a as i16) + (b as i16));
}
