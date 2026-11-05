#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// Two products by the routine in one expression: the first is kept while the second is worked out.
#[inline(never)]
fn sq(a: i16, b: i16) { out(a * a + b * b); }

pub fn main() { sq(-1, 5); sq(7, -3); sq(0, 0); }
