//! feature: `i16::try_from(a)` for a `usize`
//! section: Conversions
//! disposition: easy
//! note: At opt-level 2 rustc compares against 32767, which no card holds; every `usize` on a card passes that test.
//! input: 5 999 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    loop {
        let a = inp() as usize;
        out(match i16::try_from(a) {
            Ok(v) => v,
            Err(_) => -1,
        });
        if a == 0 {
            break;
        }
    }
}
