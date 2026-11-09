#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 {
    0
}

/// The Minesweeper count on the bordered board, its eight neighbours written out: each a constant away from the
/// square's top-left neighbour, as the tip's offsets are.
pub fn main() {
    let b = [
        [0i16, 0, 0, 0, 0, 0, 0, 0],
        [0, 0, 1, 0, 0, 0, 1, 0],
        [0, 0, 0, 0, 1, 0, 0, 0],
        [0, 0, 0, 1, 0, 0, 0, 0],
        [0, 1, 0, 0, 0, 1, 0, 0],
        [0, 0, 0, 0, 0, 0, 0, 0],
    ];
    let r = inp() as usize;
    let c = inp() as usize;
    out(b[r][c]
        + b[r][c + 1]
        + b[r][c + 2]
        + b[r + 1][c]
        + b[r + 1][c + 2]
        + b[r + 2][c]
        + b[r + 2][c + 1]
        + b[r + 2][c + 2]);
}
