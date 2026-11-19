#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// A count from `% 10`, -9..=9: the head's test stays, for the counts below 1.
#[inline(never)]
fn rem(v: i16, x: i16, t: i16) -> i16 {
    let mut t = t;
    for _ in 0..v % 10 {
        t += x;
    }
    t
}

/// Every count from below the least to past the most.
pub fn main() {
    let mut v = -11;
    while v <= 11 {
        out(rem(v, 51, -450));
        out(rem(v, -55, 459));
        v += 1;
    }
}
