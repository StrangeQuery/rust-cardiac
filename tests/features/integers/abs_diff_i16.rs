//! feature: `a.abs_diff(b)` on `i16`
//! section: Sign and absolute value
//! disposition: easy
//! note: It returns a `u16`, which the compiler lacks; on a card the answer is a subtraction and a compare.
//! input: 7 3 3 7 -500 499 -4 -9 0 5 0 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    loop {
        let a = inp();
        let b = inp();
        if a == 0 && b == 0 {
            break;
        }
        out(a.abs_diff(b) as i16);
    }
}
