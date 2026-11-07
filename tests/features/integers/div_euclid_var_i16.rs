//! feature: `a.div_euclid(b)` on `i16`
//! section: Euclidean division
//! disposition: easy
//! note: `rem_euclid` by a variable works; `div_euclid` is the same test around a divide.
//! input: 10 7 -10 7 10 -7 -10 -7 0 0
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
        out(a.div_euclid(b));
    }
}
