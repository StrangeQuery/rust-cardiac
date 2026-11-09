#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// A u8 grid, 4 rows by 3, filled with values that wrap, then read down column 2: row 4 panics.
pub fn main() {
    let mut t = [[0u8; 3]; 4];
    let mut v: u8 = 250;
    for r in 0..4 {
        for c in 0..3 {
            t[r][c] = v;
            v = v.wrapping_add(2);
        }
    }
    let mut r = 0;
    loop {
        out(t[r][2] as i16);
        r += 1;
    }
}
