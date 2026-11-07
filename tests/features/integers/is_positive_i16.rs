//! feature: `a.is_positive()` on `i16`
//! section: Sign and absolute value
//! input: 7 -7 1 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    loop {
        let a = inp();
        out(a.is_positive() as i16);
        if a == 0 {
            break;
        }
    }
}
