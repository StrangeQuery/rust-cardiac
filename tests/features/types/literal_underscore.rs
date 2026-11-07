//! feature: Literals with underscores (`1_0`, `0b_1_1`, `9_9_9`)
//! section: Integer literals
//! input: 1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    out(a + 1_0);
    out(a + 0b_1_1);
    out(9_9_9 - a);
}
