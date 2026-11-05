#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// A promise of `a && b`, and of three conditions joined: each part is a promise of its own, so the division needs
/// no check for zero or for i16::MIN / -1.
#[inline(never)]
fn ratio(a: i16, b: i16) -> i16 {
    unsafe { core::hint::assert_unchecked(a >= 1 && b >= 0) };
    b / a
}

#[inline(never)]
fn third(a: i16, b: i16) -> i16 {
    unsafe { core::hint::assert_unchecked(a >= 1 && b >= 0 && b < 1000) };
    b / a
}

pub fn main() {
    out(ratio(3, 7));
    out(ratio(5, 0));
    out(third(2, 9));
    out(third(7, 999));
}
