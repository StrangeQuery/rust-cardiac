#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 {
    0
}

/// The Minesweeper count of the tables-by-tens tip, its board 4 rows by 6 squares with no border: reads a square's
/// row and column and prints how many of its eight neighbours are mines, keeping to the board with min and max (the
/// tip's "Why not just check for the edges?").
///
/// The same program with its input promised (`assert_unchecked`): the input is always a square on the board, so
/// no bounds check can fail.
pub fn main() {
    let b = [
        [0i16, 1, 0, 0, 0, 1],
        [0, 0, 0, 1, 0, 0],
        [0, 0, 1, 0, 0, 0],
        [1, 0, 0, 0, 1, 0],
    ];
    let r = inp() as usize;
    let c = inp() as usize;
    unsafe { core::hint::assert_unchecked(r < 4) };
    unsafe { core::hint::assert_unchecked(c < 6) };
    let mut n = 0;
    for i in r.max(1) - 1..=(r + 1).min(3) {
        for j in c.max(1) - 1..=(c + 1).min(5) {
            n += b[i][j];
        }
    }
    out(n - b[r][c]);
}
