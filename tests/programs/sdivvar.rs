#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// Divide by a variable, every combination of signs: the shared routine.
#[inline(never)]
fn div(a: i16, b: i16) { out(a / b); }

pub fn main() { div(-37, -7); div(-37, 7); div(37, -7); div(37, 7); }
