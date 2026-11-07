//! feature: `usize::try_from(a)` for an `i16`
//! section: Conversions
//! disposition: easy
//! note: At opt-level 1 rustc calls the method rather than inlining it; the compiler could write what rustc inlines at opt-level 2, as it does for `min` and `max`.
//! input: 5 999 -1 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    loop {
        let a = inp();
        out(match usize::try_from(a) {
            Ok(v) => v as i16,
            Err(_) => -1,
        });
        if a == 0 {
            break;
        }
    }
}
