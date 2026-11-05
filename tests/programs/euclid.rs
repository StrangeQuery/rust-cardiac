#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// rem_euclid by a variable, which rustc calls at opt-level 1 and s: never negative, though the dividend
/// and the divisor are (-37 % -7 is -2; -2 - -7 is 5). With the division routine, one call is what fits.
#[inline(never)]
fn rem(a: i16, b: i16) { out(a.rem_euclid(b)); }

pub fn main() { rem(-37, -7); }
