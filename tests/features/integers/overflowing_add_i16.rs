//! feature: `a.overflowing_add(b)` on `i16`, on a card
//! section: Overflowing
//! disposition: easy
//! note: At opt-level 1 rustc calls the method rather than inlining it; the compiler could write what rustc inlines at opt-level 2, as it does for `min` and `max`.
//! input: 7 3 -7 3 500 499 0 0
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
        let (v, o) = a.overflowing_add(b);
        out(v);
        out(o as i16);
    }
}
