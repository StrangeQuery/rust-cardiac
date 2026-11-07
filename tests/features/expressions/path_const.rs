//! feature: A `const` used in an expression
//! section: Path expressions
//! input: 3 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
const STEP: i16 = 25;
pub fn main() {
    loop {
        let a = inp();
        if a == 0 { break; }
        out(a * STEP + STEP);
    }
}
