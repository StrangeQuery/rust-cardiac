#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// A loop of a constant count, as a `while`, with two totals worked out at once, a product from 0 and a constant
/// step from a value, and the counter read after it.
#[inline(never)]
fn both(d: i16) {
    let d = d % 10;
    let mut v = 0;
    let mut w = 5;
    let mut i = 0;
    while i < 64 {
        v += d;
        w += 1;
        i += 1;
    }
    out(v);
    out(w);
    out(i);
}

pub fn main() {
    for x in [-83, 5, 83] {
        both(x);
    }
}
