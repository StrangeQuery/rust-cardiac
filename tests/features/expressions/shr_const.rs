//! feature: `a >> 2`: a shift right by a constant
//! section: Bitwise and shift operators
//! disposition: easy
//! note: `a >> k` on an `i16` is `a.div_euclid(2^k)`, which the compiler already does for a constant.
//! input: 13 -13 100 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    loop {
        let a = inp();
        out(a >> 2);
        if a == 0 {
            break;
        }
    }
}
