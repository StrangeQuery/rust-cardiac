//! feature: `core::mem::swap`
//! section: Memory (core::mem)
//! disposition: easy
//! note: On scalars it is two reads and two writes; rustc calls it at opt-level 1 and uses an intrinsic at opt-level 2.
//! input: 3 7
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let mut a = inp();
    let mut b = inp();
    core::mem::swap(&mut a, &mut b);
    out(a);
    out(b);
}
