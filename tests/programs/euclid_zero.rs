#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// rem_euclid by zero panics, whatever the profile, as `%` does.
#[inline(never)]
fn rem(a: i16, b: i16) { out(a.rem_euclid(b)); }

pub fn main() { rem(-37, 0); }
