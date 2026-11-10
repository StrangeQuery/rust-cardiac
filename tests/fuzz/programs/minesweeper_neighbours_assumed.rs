use cardiac::*;

/// The Minesweeper count on the bordered board, its eight neighbours written out: each a constant away from the
/// square's top-left neighbour, as the tip's offsets are.
///
/// The same program with its input promised (`assume`): the input is always a square on the board, so
/// no bounds check can fail.
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
    unsafe { assume(r < 4) };
    unsafe { assume(c < 6) };
    out(b[r][c]
        + b[r][c + 1]
        + b[r][c + 2]
        + b[r + 1][c]
        + b[r + 1][c + 2]
        + b[r + 2][c]
        + b[r + 2][c + 1]
        + b[r + 2][c + 2]);
}
