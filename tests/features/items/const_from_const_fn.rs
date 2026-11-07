//! feature: A `const` from a `const fn`
//! section: Constant items
//! disposition: easy
//! note: rustc has already run the `const fn`; the compiler only has to read the array it made.
//! input: 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
const fn table() -> [i16; 5] {
    let mut t = [0; 5];
    let mut i = 0;
    while i < 5 { t[i] = (i as i16) * (i as i16); i += 1; }
    t
}
const SQ: [i16; 5] = table();
pub fn main() {
    let k = inp() as usize;
    for i in k..5 { out(SQ[i]); }
}
