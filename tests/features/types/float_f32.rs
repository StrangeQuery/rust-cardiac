//! feature: `f32` arithmetic, cast back to `i16`
//! section: Floating-point types
//! disposition: never
//! input: 7 2
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp() as f32;
    let b = inp() as f32;
    out((a / b * 10.0) as i16);
}
