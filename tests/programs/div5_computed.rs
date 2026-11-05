#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// A quotient by 5 and by 25 into an element at a variable index: adds and a shift, as into a local.
#[inline(never)]
fn into(i: usize, x: i16) {
    let mut a: [i16; 3] = [100, 2, 3];
    a[i] = x / 5;
    a[2 - i] = x / 25;
    out(a[0]);
    out(a[1]);
    out(a[2]);
}

pub fn main() { into(0, -999); into(2, 998); into(1, 7); }
