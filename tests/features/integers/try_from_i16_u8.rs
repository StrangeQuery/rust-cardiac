//! feature: `u8::try_from(a)` for an `i16`
//! section: Conversions
//! disposition: easy
//! note: It is a range check (0 to 255) and then the cast.
//! input: 5 255 256 -1 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    loop {
        let a = inp();
        out(match u8::try_from(a) {
            Ok(v) => v as i16,
            Err(_) => -1,
        });
        if a == 0 {
            break;
        }
    }
}
