#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// A signed remainder through a &mut: the shared routine.
#[inline(never)]
fn rem(r: &mut i16, x: i16, y: i16) { *r = x % y; }

pub fn main() {
    let mut v: i16 = 1;
    rem(&mut v, -37, 7);
    out(v);
}
