#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 {
    0
}

/// The Minesweeper count of the tables-by-tens tip on its board inside a border of empty squares the program
/// declares, 6 rows by 8 (the tip's "C-style, with a border"): reads a square's row and column on the board and
/// adds up the 3 by 3 block around it, less the square itself, with no tests for the edges.
///
/// The same program with its input promised (`assert_unchecked`): the input is always a square on the board, so
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
    unsafe { core::hint::assert_unchecked(r < 4) };
    unsafe { core::hint::assert_unchecked(c < 6) };
    let mut n = 0;
    for i in r..r + 3 {
        for j in c..c + 3 {
            n += b[i][j];
        }
    }
    out(n - b[r + 1][c + 1]);
}
