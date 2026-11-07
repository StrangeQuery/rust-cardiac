//! feature: `u8::MAX`, `i8::MIN` and `i8::MAX` used with an input
//! section: Integer types
//! input: 200 -100
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    let b = inp();
    out((a < u8::MAX as i16) as i16);
    out((b > i8::MIN as i16) as i16);
    out(u8::MAX as i16 - a);
    out(i8::MAX as i16 + b);
}
