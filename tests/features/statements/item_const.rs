//! feature: a `const` declared inside a function
//! section: Item declarations
//! input: 7
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    const STEP: i16 = 25;
    let a = inp();
    out(a + STEP);
}
