//! feature: `a = a.div_euclid(b)`, the result in the dividend's own cell
//! section: Euclidean division
//! input: -10 7 -100 -7 50 -3 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    loop {
        let mut a = inp();
        if a == 0 {
            break;
        }
        let mut b = inp();
        let c = a;
        a = a.div_euclid(b);
        out(a);
    }
}
