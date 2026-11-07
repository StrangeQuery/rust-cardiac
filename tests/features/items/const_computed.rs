//! feature: A `const` computed from other constants
//! section: Constant items
//! disposition: easy
//! note: rustc has already worked out the array; the compiler only has to read its value, as it does for a literal table.
//! input: 1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
const A: i16 = 12;
const B: i16 = A * 7 + 3;
const N: usize = 3;
const T: [i16; N] = [A, B, A + B];
pub fn main() {
    out(T[inp() as usize]);
    out(B);
}
