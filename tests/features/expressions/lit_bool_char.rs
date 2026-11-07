//! feature: `true`, `false` and a `char` literal cast to a number (`'A' as i16`)
//! section: Literal expressions
//! disposition: easy
//! note: A `char` is a number, its code point, and one that fits on a card can be held as a `u8` is.
//! input: 1 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp() != 0;
    let b = inp() != 0;
    out((a == true) as i16);
    out((b == false) as i16);
    let c = if a { 'A' } else { 'B' };
    out(c as i16);
}
