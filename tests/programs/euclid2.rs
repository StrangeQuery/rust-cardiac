#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// rem_euclid by a variable, which rustc calls at opt-level 1 and s: never negative, though the dividend is:
/// -36 % 7 is -1, and -1 + 7 is 6. With the division routine, one call is what fits.
#[inline(never)]
fn rem(a: i16, b: i16) { out(a.rem_euclid(b)); }

pub fn main() { rem(-36, 7); }
