//! feature: `a.clamp(lo, hi)` on `i16`
//! section: Min, max and clamp
//! input: 5 0 10 -5 0 10 50 0 10 0 0 10
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    loop {
        let a = inp();
        let lo = inp();
        let hi = inp();
        out(a.clamp(lo, hi));
        if a == 0 {
            break;
        }
    }
}
