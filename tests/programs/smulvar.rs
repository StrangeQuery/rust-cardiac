#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// Multiply by a variable: the shared routine, repeated addition, signs put back.
pub fn main() {
    let xs: [i16; 4] = [-31, 12, 0, -1];
    let mut j = 0usize;
    while j < 3 { out(xs[j] * xs[j + 1]); j += 1; }
    let mut k: i16 = 1;
    while k < 300 { k = k * 3 - 1; out(k); }
}
