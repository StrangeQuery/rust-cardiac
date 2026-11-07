//! feature: `u8::MIN` and `u8::MAX`
//! section: Constants
//! input: 1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    out(u8::MIN as i16 + a);
    out(u8::MAX as i16 - a);
}
