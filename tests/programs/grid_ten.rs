#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// A grid 10 wide, already by tens: filled, read, and read past its last row, which panics.
pub fn main() {
    let mut ten = [[0i16; 10]; 2];
    for r in 0..2 {
        for c in 0..10 {
            ten[r][c] = (r * 10 + c) as i16;
        }
    }
    out(ten[1][9] + ten[1][0]);
    let mut r = 0;
    loop {
        out(ten[r][4]);
        r += 1;
    }
}
