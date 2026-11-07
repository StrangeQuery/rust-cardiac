//! feature: `i32`: declared, added, subtracted, compared and cast back to `i16`
//! section: Integer types
//! note: The values fit on a card here; an `i32` past a card is its own row.
//! input: 400 -300
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a: i32 = inp() as i32;
    let b: i32 = inp() as i32;
    out((a + b) as i16);
    out((a - b) as i16);
    out((a < b) as i16);
    out((a == b) as i16);
}
