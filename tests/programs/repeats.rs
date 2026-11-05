#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// A helper called again with the same value: once inlined, each size is worked out only once.
fn size(v: i16) -> i16 { if v < 0 { -v } else { v } }

#[inline(never)]
fn far(a: i16, b: i16) -> i16 {
    if size(a) < size(b) { size(b) } else { size(a) }
}

pub fn main() {
    out(far(-7, 3)); out(far(4, -9)); out(far(5, -5)); out(far(0, 2));
}
