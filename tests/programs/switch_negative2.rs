#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

#[inline(never)]
fn word(x: i16) -> i16 {
    match x {
        -999 => 9,
        -2 => 2,
        -1 => 1,
        _ => 5,
    }
}

/// The negative arms of a switchInt, as a `match` writes them, and a loop variable compared with a negative
/// constant: an i32's, whose -1 MIR prints as 4294967295, in a loop that runs and in one over an empty range.
pub fn main() {
    let xs: [i16; 3] = [-1, -999, 4];
    let mut k: usize = 0;
    while k < 3 {
        out(word(xs[k]));
        k += 1;
    }
    for i in -3..0 {
        if i == -2 {
            out(-2);
        }
    }
    for i2 in 0..0 {
        if i2 == -1 {
            out(0);
        }
    }
}
