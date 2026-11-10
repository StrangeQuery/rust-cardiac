use cardiac::*;

/// The Minesweeper count on the bordered board, the square read as one number, its row then its column (20 for row
/// 2, column 0), as in the tip's "By tens, one number".
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
    let s = inp() as usize;
    unsafe { assume(s < 36) };
    unsafe { assume(s % 10 < 6) };
    let (r, c) = (s / 10, s % 10);
    let mut n = 0;
    for i in r..r + 3 {
        for j in c..c + 3 {
            n += b[i][j];
        }
    }
    out(n - b[r + 1][c + 1]);
}
