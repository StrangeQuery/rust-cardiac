#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// `rem_euclid(2)` and `rem_euclid(5)`: the remainder by the last digit, and the divisor added to a negative one.
#[inline(never)]
fn rem(x: i16) {
    out(x.rem_euclid(2));
    out(x.rem_euclid(5));
}

pub fn main() {
    for x in [0, -1, 4, 7, -7, 123, 999, -999] {
        rem(x);
    }
}
