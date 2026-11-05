#![no_std]
#[inline(never)]
pub fn out(_v: u8) {}

/// Sums an array it borrows. Indexing is bounds-checked.
#[inline(never)]
pub fn sum(xs: &[u8; 5]) -> u8 {
    let mut s: u8 = 0;
    let mut i: usize = 0;
    while i < 5 {
        s += xs[i];
        i += 1;
    }
    s
}

pub fn main() {
    let mut a: [u8; 5] = [3, 1, 4, 1, 5];
    let mut i: usize = 0;
    while i < 5 {
        out(a[i]);
        i += 1;
    }
    a[2] = 9;
    out(sum(&a));
}
