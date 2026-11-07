//! feature: `u8::try_from(a)` for a `usize`
//! section: Conversions
//! disposition: easy
//! note: At opt-level 1 rustc calls the method rather than inlining it; the compiler could write what rustc inlines at opt-level 2, as it does for `min` and `max`.
//! input: 5 255 256 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    loop {
        let a = inp() as usize;
        out(match u8::try_from(a) {
            Ok(v) => v as i16,
            Err(_) => -1,
        });
        if a == 0 {
            break;
        }
    }
}
