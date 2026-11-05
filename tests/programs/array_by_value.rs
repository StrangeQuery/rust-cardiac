#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// Sums an array it is given by value: every element must be copied into its parameter, not only the first.
#[inline(never)]
fn sum(values: [i16; 4]) -> i16 {
    let mut total = 0;
    let mut i = 0;
    while i < 4 {
        total += values[i];
        i += 1;
    }
    total
}

pub fn main() {
    let values = [7, 3, 1, 2];
    out(sum(values));
    out(values[3]);
}
