#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

#[inline(never)]
fn mk(v: i16) -> [i16; 3] {
    [v, 5, -v]
}

/// A whole array copied, out of a callee's result and from one local into another: every element, not only
/// the first. `b` is written at a computed index, so rustc keeps the copy.
#[inline(never)]
fn f(a: [i16; 3], i: usize) {
    let mut b = a;
    b[i] = 1;
    out(a[i]);
    out(b[2]);
}

pub fn main() {
    let a = mk(4);
    out(a[2]);
    f(a, 0);
    f(a, 2);
}
