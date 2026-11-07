//! feature: `a.min(b)` on `i16`
//! section: Min, max and clamp
//! input: 7 3 -7 3 5 5 -999 999 0 0
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
        out(a.min(b));
    }
}
