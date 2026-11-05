#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// Multiply by constants: adds and decimal shifts, checked so a partial product never carries past four digits.
pub fn main() {
    let xs: [i16; 4] = [-27, 0, 9, 23];
    let mut j = 0usize;
    while j < 4 {
        let x = xs[j];
        out(x * -2); out(-x); out(x * 37);
        j += 1;
    }
    out(xs[2] * -100);
}
