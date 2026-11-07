//! feature: `!a` on `i16` (bitwise not)
//! section: Negation operators
//! disposition: easy
//! note: On a signed integer `!a` is `-a - 1`, which CARDIAC can do in two instructions.
//! input: 7 -7 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    loop {
        let a = inp();
        out(!a);
        if a == 0 {
            break;
        }
    }
}
