#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// Linear folding: v is a sum, `x + (x - y)`, with the temporary folded in, then assigned again in the same
/// block, `v -= x`. The second sum replaced the first one's record before it was rewritten, so `x - y` was dropped
/// and its sum was left reading a temporary never stored: 0 for 1 - 0.
#[inline(never)]
fn twice(x: i16, y: i16) {
    let mut v = x + (x - y);
    v -= x;
    out(v);
}

/// With overflow checks too, once a promise rules them out: the ops are plain, and folded the same way (rustc
/// leaves `one += 0` for the program to work out).
#[inline(never)]
fn promised(x: i16, y: i16) {
    unsafe { core::hint::assert_unchecked(x >= -5 && x <= 20 && y >= 0 && y <= 9) };
    let mut one: i16 = 1;
    one += 0;
    let mut v = (one - x) + y;
    v += one;
    out(v);
}

pub fn main() {
    twice(1, 0);
    twice(5, 2);
    twice(-7, 9);
    promised(0, 0);
    promised(-5, 9);
    promised(20, 3);
}
