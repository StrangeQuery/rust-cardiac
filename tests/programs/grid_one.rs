#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// A grid 1 wide, which stays row after row (its row is already its index): filled, and read past its end, which
/// panics.
pub fn main() {
    let mut one = [[0i16; 1]; 4];
    for r in 0..4 {
        one[r][0] = r as i16 + 1;
    }
    let mut r = 1;
    loop {
        out(one[r][0]);
        r += 1;
    }
}
