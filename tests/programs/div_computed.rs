#![no_std]
#[inline(never)]
pub fn out(_v: u8) {}

/// An unsigned quotient by a variable into an element at a variable index: the subtracting loop, into a scratch
/// cell and stored there. The last divisor is zero, which panics.
#[inline(never)]
fn into(i: usize, x: u8, y: u8) {
    let mut a: [u8; 2] = [100, 2];
    a[i] = x / y;
    out(a[0]);
    out(a[1]);
}

pub fn main() { into(0, 200, 7); into(1, 9, 9); into(1, 5, 0); }
