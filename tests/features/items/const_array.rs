//! feature: A `const` array indexed at run time
//! section: Constant items
//! input: 2 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
const T: [i16; 4] = [10, 20, 30, 40];
pub fn main() {
    out(T[inp() as usize]);
    out(T[inp() as usize]);
}
