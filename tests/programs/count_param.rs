#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

#[inline(never)]
fn id(v: i16) -> i16 {
    v
}

/// The bound is a parameter: each call sets it afresh, so the loop may count it down.
#[inline(never)]
fn sum(n: i16, step: i16) -> i16 {
    let mut s = 0;
    for _ in 0..n {
        s += step;
    }
    s
}

/// `for _ in 0..n` with n a parameter, called again and again; and a bound read after its loop, which must keep its
/// value, so that one is not counted down.
pub fn main() {
    out(sum(3, 2));
    out(sum(0, 9));
    out(sum(4, 1));
    let k = id(3);
    let mut total = 0;
    for _ in 0..k {
        total += 1;
    }
    out(total + k);
}
