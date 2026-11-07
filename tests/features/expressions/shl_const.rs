//! feature: `a << 2`: a shift left by a constant
//! section: Bitwise and shift operators
//! disposition: easy
//! note: `a << k` is `a * 2^k`, which the compiler already does for a constant.
//! input: 3 -3 100 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    loop {
        let a = inp();
        out(a << 2);
        if a == 0 {
            break;
        }
    }
}
