#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// A grid filled, copied whole and changed, and the copy read past its row's end, which panics.
pub fn main() {
    let mut t = [[0i16; 3]; 3];
    for r in 0..3 {
        for c in 0..3 {
            t[r][c] = (r * 3 + c) as i16;
        }
    }
    let mut u = t;
    u[1][1] = -1;
    out(t[1][1]);
    out(u[1][1]);
    let mut c = 0;
    loop {
        out(u[2][c]);
        c += 1;
    }
}
