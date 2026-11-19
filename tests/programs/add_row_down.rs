#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn digit(v: i16, k: i16) -> i16 { if k == 0 { v / 100 } else if k == 1 { v / 10 % 10 } else { v % 10 } }

/// Counting down, `while n > 0`, n a digit.
#[inline(never)]
fn down(v: i16, x: i16) -> i16 {
    let mut n = digit(v, 1);
    let mut t = 5;
    while n > 0 {
        t -= x;
        n -= 1;
    }
    t
}

/// Every count from below the least to past the most.
pub fn main() {
    let mut v = -11;
    while v <= 11 {
        out(down(v * 10, 51));
        out(down(v * 10, -55));
        v += 1;
    }
}
