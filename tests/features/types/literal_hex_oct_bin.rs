//! feature: Hexadecimal, octal and binary literals (`0x1F`, `0o17`, `0b101`)
//! section: Integer literals
//! input: 100
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    out(a + 0x1F);
    out(a + 0o17);
    out(a + 0b101);
    out(a - 0xff);
}
