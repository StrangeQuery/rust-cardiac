//! feature: `u16`: declared, added, subtracted, compared and cast back to `i16`
//! section: Integer types
//! disposition: demand
//! note: Every `u16` on a card is fine and one past a card can't be held, as for `i16`.
//! input: 400 300
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a: u16 = inp() as u16;
    let b: u16 = inp() as u16;
    out((a + b) as i16);
    out((a - b) as i16);
    out((a < b) as i16);
    out((a == b) as i16);
}
