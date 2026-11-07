//! feature: `core::mem::replace`
//! section: Memory (core::mem)
//! disposition: easy
//! note: At opt-level 1 rustc calls it; on a scalar it is a read and a write.
//! input: 3 7
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let mut a = inp();
    let b = inp();
    let old = core::mem::replace(&mut a, b);
    out(old);
    out(a);
}
