#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

#[inline(never)]
pub fn id(x: i16) -> i16 {
    x
}

#[inline(never)]
pub fn get(x: &mut i16) -> i16 {
    *x
}

/// addr_out.rs through a `&mut`, which stays an address (a shared `&i16` is passed as its value): n is
/// stored, printed, then read through its address, so the store must stay.
#[inline(never)]
pub fn g(v: i16) {
    let mut n = id(v);
    out(n);
    out(get(&mut n));
}

pub fn main() {
    g(40);
    g(7);
}
