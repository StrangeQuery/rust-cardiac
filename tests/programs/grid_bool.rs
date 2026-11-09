#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// A bool grid, with an index changed between two uses: then read past its last row, which panics.
pub fn main() {
    let mut seen = [[false; 4]; 3];
    seen[1][3] = true;
    seen[0][2] = true;
    seen[2][0] = true;
    let mut i = 1;
    out(seen[i][3] as i16 + seen[0][i] as i16);
    i += 1;
    out(seen[i][0] as i16 + seen[0][i] as i16);
    let mut r = 0;
    loop {
        out(seen[r][2] as i16);
        r += 1;
    }
}
