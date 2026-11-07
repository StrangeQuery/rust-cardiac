//! feature: `a.checked_sub(b)` on `usize`
//! section: Checked
//! disposition: easy
//! note: At opt-level 1 rustc calls the method rather than inlining it; the compiler could write what rustc inlines at opt-level 2, as it does for `min` and `max`.
//! input: 7 3 3 7 5 5 0 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    loop {
        let a = inp() as usize;
        let b = inp() as usize;
        if a == 0 && b == 0 {
            break;
        }
        match a.checked_sub(b) {
            Some(v) => out(v as i16),
            None => out(-1),
        }
    }
}
