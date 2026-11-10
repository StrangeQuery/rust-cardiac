//! Neighbours of a square on a small board, each a constant away from the square, as Minesweeper reads them: the
//! bounds checks of one index one after another, which the compiler may make one. Off the board panics both ways.
use cardiac::*;

fn main() {
    let b = [[1i16, 2, 3], [4, 5, 6], [7, 8, 9]];
    let r = inp() as usize;
    let c = inp() as usize;
    out(b[r][c] + b[r][c + 1] + b[r + 2][c] + b[r + 1][c + 2]);
}
