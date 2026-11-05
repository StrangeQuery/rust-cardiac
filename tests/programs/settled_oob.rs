#![no_std]
#![allow(unconditional_panic)]
#[inline(never)]
pub fn out(_v: i16) {}

/// An index rustc has already found past the end (`assert(const false, ..)`): the bounds check fails every time, so
/// the program panics there, after what it printed before, and nothing after it is compiled.
pub fn main() {
    let mut a: [i16; 3] = [9, 1, 4];
    out(a[0] + a[2]);
    a[1] = 7;
    out(a[3]);
    out(a[1]);
}
