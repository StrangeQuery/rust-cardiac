#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// A count tested before the loop, a `while` with its counter read after it, and x + x a pass.
#[inline(never)]
fn tested(d: i16, x: i16) -> i16 {
    let mut t = 0;
    if d >= 0 && d < 8 {
        let mut k = 0;
        while k < d {
            t += x + x;
            k += 1;
        }
        out(k);
    }
    t
}

/// Every count from below the least to past the most.
pub fn main() {
    let mut v = -11;
    while v <= 11 {
        out(tested(v, 51));
        out(tested(v, -55));
        v += 1;
    }
}
