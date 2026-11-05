#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// Remainder by a variable, every combination of signs: the shared routine (it keeps the dividend's sign).
#[inline(never)]
fn rem(a: i16, b: i16) { out(a % b); }

pub fn main() { rem(-37, -7); rem(-37, 7); }   // 37 % -7: sremvar2 (both will not fit with the routine)
