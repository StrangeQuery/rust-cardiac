//! feature: `a.wrapping_neg()` on `i16`
//! section: Wrapping
//! input: 7 -7 999 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    loop {
        let a = inp();
        out(a.wrapping_neg());
        if a == 0 {
            break;
        }
    }
}
