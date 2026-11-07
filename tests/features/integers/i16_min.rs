//! feature: `i16::MIN`
//! section: Constants
//! disposition: never
//! note: Refused on purpose (RCE0009), as other constants past a card are; comparing with one works.
//! input: 1
//! differs: `i16::MIN` is -32768 in Rust, which no card holds; on CARDIAC an `i16` ends at -999.
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    out((i16::MIN + a) / 100);
}
