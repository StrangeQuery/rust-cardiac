//! feature: `a.div_euclid(k)` on `i16` by 10
//! section: Euclidean division
//! input: 10 -10 7 -7 -1 0 1 -999 999 998
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    loop {
        let a = inp();
        if a == 998 {
            break;
        }
        out(a.div_euclid(10));
    }
}
