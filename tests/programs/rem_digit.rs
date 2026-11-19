#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// `% 2` and `% 5` by the last digit, every sign: Rust's remainder takes the dividend's.
#[inline(never)]
fn rem(x: i16) {
    out(x % 2);
    out(x % 5);
    out(x % -5);
}

pub fn main() {
    for x in [0, -1, 4, 7, -7, 123, 999, -999] {
        rem(x);
    }
}
