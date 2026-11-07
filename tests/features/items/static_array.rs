//! feature: A `static` array indexed at run time
//! section: Static items
//! disposition: easy
//! note: An immutable `static` is a table in fixed cells, as a `const` array already is.
//! input: 1 3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
static T: [i16; 4] = [5, -6, 7, -8];
pub fn main() {
    out(T[inp() as usize]);
    out(T[inp() as usize]);
}
