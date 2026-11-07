//! feature: `a.ilog10()` on `i16`
//! section: Powers and roots
//! disposition: easy
//! note: On a card it is two compares (against 10 and 100); rustc works in `u16` and `u32`, which the compiler lacks.
//! input: 1 9 10 99 100 999 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    loop {
        let a = inp();
        if a == 0 {
            break;
        }
        out(a.ilog10() as i16);
    }
}
