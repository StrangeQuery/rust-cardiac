//! feature: Integer literals: decimal, `0x`, `0o`, `0b`, with `_` separators and a type suffix
//! section: Literal expressions
//! input: 1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    out(a * 0x1F);
    out(a + 0o17);
    out(a + 0b101);
    out(a + 1_0_0);
    out(a + 7i16);
}
