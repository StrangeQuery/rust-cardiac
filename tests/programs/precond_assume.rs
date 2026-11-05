#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// `core::hint::assert_unchecked` of a false condition: with debug assertions on, core's check of its precondition
/// panics, where rustc has inlined it (opt-level 2) and where it is a call of core's (opt-level 1).
#[inline(never)]
fn half(a: i16) -> i16 {
    unsafe { core::hint::assert_unchecked(a >= 0) };
    a / 2
}

pub fn main() {
    out(half(8));
    out(half(-8));
}
