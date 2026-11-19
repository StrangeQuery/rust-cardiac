#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// A difference a pass, on a card, worked out once; and a counter that counts too.
#[inline(never)]
fn difference(d: i16, a: i16, b: i16) -> i16 {
    let (a, b) = (a % 100, b % 100);
    let mut t = 0;
    let mut q = 0;
    for _ in 0..d % 10 {
        t += a - b;
        q += 1;
    }
    out(q);
    t
}

/// Every count from below the least to past the most.
pub fn main() {
    let mut v = -11;
    while v <= 11 {
        out(difference(v, 51, -55));
        out(difference(v, -55, 7));
        v += 1;
    }
}
