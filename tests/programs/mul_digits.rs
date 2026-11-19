#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// Products of two variables, by the multiply routine digit by digit: every sign, multipliers of one, two and
/// three digits, zeros among their digits, and products up to 999.
#[inline(never)]
fn mul(a: i16, b: i16) {
    out(a * b);
}

const A: [i16; 8] = [7, 1, -999, -1, 37, -9, 4, -5];
const B: [i16; 8] = [0, 999, 1, -999, -27, 101, 249, 190];

pub fn main() {
    for i in 0..8 {
        mul(A[i], B[i]);
    }
}
