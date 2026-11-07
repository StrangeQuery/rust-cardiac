//! feature: `u8::MAX` as a value
//! section: Path expressions
//! input: 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    out(a + u8::MAX as i16);
}
