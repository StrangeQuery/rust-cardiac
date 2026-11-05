#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
/// The add_times() intrinsic: on CARDIAC a computed jump into a shared row of ADD c, for the tens and then
/// the units of n.
#[inline(never)]
pub fn add_times(c: i16, n: i16) -> i16 {
    let mut sum = 0;
    let mut k = 0;
    while k < n {
        sum += c;
        k += 1;
    }
    sum
}

/// The counts the issue asks for, 0 to the largest (99), and more, each with a c whose product fits on a card.
const CS: [i16; 8] = [3, -7, -111, 99, -27, 10, 999, 0];
const NS: [i16; 8] = [0, 1, 9, 10, 37, 99, 1, 55];

pub fn main() {
    // c and n read through their addresses
    for i in 0..8 {
        out(add_times(CS[i], NS[i]));
    }
}
