#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// A row of 3 squares inside a border the program declares itself, of tens above, of 100s at the sides and of 200s
/// below, so that each border cell read shows: for each square, its neighbours above and below, then left and right
/// (its row from black_box, so that rustc keeps the grid whole). Then the square past the end, whose neighbour to
/// the right is past the border: that panics.
pub fn main() {
    let b = [[10i16, 20, 30, 40, 50], [100, 1, 2, 3, 100], [200, 210, 220, 230, 240]];
    let r = core::hint::black_box(1);
    let mut c = 1;
    loop {
        out(b[r - 1][c] + b[r + 1][c]);
        out(b[r][c - 1] + b[r][c + 1]);
        c += 1;
    }
}
