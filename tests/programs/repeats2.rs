#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// A sum worked out before a branch and again on one side of it: the second is reused. The sum after `a`
/// changes is worked out afresh.
#[inline(never)]
fn again(a: i16, b: i16) -> i16 {
    let s = a + b;
    let t = if a < 0 { a + b } else { b - a };
    let mut c = a;
    c += 1;
    s - t + (c + b)
}

pub fn main() {
    out(again(-7, 3)); out(again(4, -9)); out(again(0, 5));
}
