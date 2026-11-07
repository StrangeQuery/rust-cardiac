//! feature: `a.signum()` on `i16`
//! section: Sign and absolute value
//! disposition: easy
//! note: At opt-level 2 rustc inlines it into a three-way compare (`Cmp`), which is two tests on CARDIAC.
//! input: 7 -7 1 -1 999 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    loop {
        let a = inp();
        out(a.signum());
        if a == 0 {
            break;
        }
    }
}
