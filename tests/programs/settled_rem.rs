#![no_std]
#![allow(unconditional_panic)]
#[inline(never)]
pub fn out(_v: i16) {}

/// A remainder by the constant zero: rustc has already worked out its check (`assert(!const true, ..)`), which
/// fails every time.
pub fn main() {
    let mut i: i16 = 0;
    while i < 3 {
        out(i % 2);
        i += 1;
    }
    out(i % 0);
    out(i);
}
