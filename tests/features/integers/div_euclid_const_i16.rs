//! feature: `a.div_euclid(7)` on `i16`
//! section: Euclidean division
//! input: 10 -10 -7 999 -999 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    loop {
        let a = inp();
        out(a.div_euclid(7));
        if a == 0 {
            break;
        }
    }
}
