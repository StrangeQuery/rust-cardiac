//! feature: `a.try_into()` from `i16` to `u8`
//! section: Conversions
//! disposition: easy
//! note: As for `u8::try_from`.
//! input: 5 255 256 -1 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    loop {
        let a = inp();
        let r: Result<u8, _> = a.try_into();
        match r {
            Ok(v) => out(v as i16),
            Err(_) => out(-1),
        }
        if a == 0 {
            break;
        }
    }
}
