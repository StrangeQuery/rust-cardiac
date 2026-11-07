//! feature: `Option<&i16>::copied`
//! section: Option
//! disposition: demand
//! note: An `Option<&i16>` holds a reference, and an enum's fields must be scalars for now.
//! input: 5 -1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    for _ in 0..2 {
        let a = inp();
        let r: Option<&i16> = if a > 0 { Some(&a) } else { None };
        out(r.copied().unwrap_or(0));
    }
}
