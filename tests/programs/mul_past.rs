#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// Products of two variables past a card: with overflow checks CARDIAC stops at the first, without them the
/// result is the last three digits, with the sign, as the cells of the multiply routine keep them.
#[inline(never)]
fn mul(a: i16, b: i16) {
    out(a * b);
}

const A: [i16; 7] = [12, 500, -500, 25, -7, 10, 999];
const B: [i16; 7] = [12, 3, 3, 41, 143, 100, -999];

pub fn main() {
    for i in 0..7 {
        mul(A[i], B[i]);
    }
}
