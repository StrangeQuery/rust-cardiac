#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// A signed quotient into an element at a variable index: the shared routine.
#[inline(never)]
fn into(i: usize, x: i16, y: i16) {
    let mut a: [i16; 2] = [100, 2];
    a[i] = x / y;
    out(a[0]);
    out(a[1]);
}

pub fn main() { into(1, -37, 7); }
