#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// A promised count, 0..=9: no test at all, 0 jumps to the row's end.
#[inline(never)]
fn promised(d: i16, x: i16, t: i16) -> i16 {
    unsafe { core::hint::assert_unchecked(d >= 0 && d <= 9) };
    let mut t = t;
    for _ in 0..d {
        t += x;
    }
    t
}

/// Every count from below the least to past the most.
pub fn main() {
    let mut v = -11;
    while v <= 11 {
        if v >= 0 && v <= 9 {
            out(promised(v, 51, -450));
            out(promised(v, -55, 459));
        }
        v += 1;
    }
}
