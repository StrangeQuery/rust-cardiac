//! feature: `f64` arithmetic and comparison
//! section: Floating-point types
//! disposition: never
//! input: 7 2
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp() as f64;
    let b = inp() as f64;
    out((a * 1.5 + b) as i16);
    out((a < b) as i16);
}
