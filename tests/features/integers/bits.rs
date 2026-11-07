//! feature: `i16::BITS`, `u8::BITS` and `usize::BITS`
//! section: Constants
//! input: 1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    out(i16::BITS as i16 * a);
    out(u8::BITS as i16 * a);
    out(usize::BITS as i16 * a);
}
