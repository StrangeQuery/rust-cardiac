//! feature: A `char` from a `u8`, compared and cast back
//! section: Textual types
//! disposition: easy
//! note: An ASCII `char` fits on a card; one past 999 never would.
//! input: 65 98
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp() as u8 as char;
    let b = inp() as u8 as char;
    out(a as i16);
    out((a < b) as i16);
    out((b == 'b') as i16);
}
