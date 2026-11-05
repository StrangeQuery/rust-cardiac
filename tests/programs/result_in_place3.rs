#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// A tuple result worked in its own cells, its first half a call's.
#[inline(never)]
fn step(x: i16) -> i16 {
    x / 2 + 3
}

#[inline(never)]
fn pair(a: i16, b: i16) -> (i16, i16) {
    let mut lo = a;
    let mut hi = b;
    if lo > hi {
        lo = b;
        hi = step(a);
    }
    (hi, lo)
}

pub fn main() {
    let mut v = -2;
    while v < 20 {
        let (h, l) = pair(v, 12);
        out(h - l);
        v += 4;
    }
}
