//! feature: `i16::MAX`
//! section: Constants
//! disposition: never
//! note: Refused on purpose (RCE0009), as other constants past a card are; comparing with one works.
//! input: 1
//! differs: `i16::MAX` is 32767 in Rust, which no card holds; on CARDIAC an `i16` ends at 999.
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    out((i16::MAX - a) / 100);
}
