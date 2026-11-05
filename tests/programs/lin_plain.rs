#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// a*5 + b*6 has coefficients adding up to 11, too much to fold; adding c to it afterwards must still see it.
#[inline(never)]
pub fn f(a: i16, b: i16, c: i16) -> i16 {
    (a * 5 + b * 6) + c
}

pub fn main() {
    out(f(1, 2, 3));
    out(f(4, 5, 6));
}
