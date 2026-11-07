//! feature: A `const` scalar
//! section: Constant items
//! input: 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
const LIMIT: i16 = 900;
pub fn main() {
    out(LIMIT - inp());
}
