#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// A grid 11 wide, which can't go by tens and stays row after row: filled, read, and read past its end, which
/// panics.
pub fn main() {
    let mut wide = [[0i16; 11]; 2];
    for c in 0..11 {
        wide[1][c] = c as i16;
    }
    out(wide[1][10] + wide[0][3]);
    let mut c = 8;
    loop {
        out(wide[1][c]);
        c += 1;
    }
}
