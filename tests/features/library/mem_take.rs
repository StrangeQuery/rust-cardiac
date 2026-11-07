//! feature: `core::mem::take`
//! section: Memory (core::mem)
//! disposition: easy
//! note: At opt-level 1 rustc calls it; on a scalar it is a read and a write of 0.
//! input: 3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let mut a = inp();
    let old = core::mem::take(&mut a);
    out(old);
    out(a);
}
