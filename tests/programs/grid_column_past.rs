#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// A grid of 3 rows by 4, filled, then row 1 read along: column 4 is past the row's end, and panics, though by tens
/// the cell it would be is on the grid's line.
pub fn main() {
    let mut t = [[0i16; 4]; 3];
    let mut r = 0;
    while r < 3 {
        let mut c = 0;
        while c < 4 {
            t[r][c] = (r * 10 + c) as i16;
            c += 1;
        }
        r += 1;
    }
    let mut c = 0;
    loop {
        out(t[1][c]);
        c += 1;
    }
}
