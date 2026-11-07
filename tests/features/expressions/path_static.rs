//! feature: Reading a `static`
//! section: Path expressions
//! input: 3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
static BASE: i16 = 40;
pub fn main() {
    let a = inp();
    out(a + BASE);
}
