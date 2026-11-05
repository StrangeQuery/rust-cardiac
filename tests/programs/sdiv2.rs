#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// Divide by 2, 5, 20, 25 and 50: multiplied up and shifted back, exact and toward zero.
pub fn main() {
    let xs: [i16; 5] = [-999, -51, 7, 49, 999];
    let mut i = 0usize;
    while i < 5 {
        let x = xs[i];
        out(x / 2); out(x / 5); out(x / 20); out(x / 25); out(x / 50);
        i += 1;
    }
}
