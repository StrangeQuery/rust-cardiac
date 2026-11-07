//! feature: `==` between two arrays
//! section: Arrays and slices
//! disposition: easy
//! note: Element by element, as `a[0] == b[0] && ...`; at opt-level 2 rustc compares the bytes as a `u64`, which the compiler would have to recognise.
//! input: 3 -1 4 1 3 -1 4 2
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let t: [i16; 4] = [inp(), inp(), inp(), inp()];
    let u: [i16; 4] = [inp(), inp(), inp(), inp()];
    out((t == t) as i16);
    out((t == u) as i16);
}
