#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

#[inline(never)]
pub fn id(x: i16) -> i16 {
    x
}

#[inline(never)]
pub fn get(x: &i16) -> i16 {
    *x
}

/// n is stored, printed, then read through its address: the store must stay.
#[inline(never)]
pub fn g(v: i16) {
    let n = id(v);
    out(n);
    out(get(&n));
}

pub fn main() {
    g(40);
    g(7);
}
