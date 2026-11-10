use cardiac::*;

/// The neighbours' offsets from a square's top-left neighbour, a row and a column each: the 3 by 3 block around it
/// less 11, the square itself.
const OFFSETS: [usize; 8] = [0, 1, 2, 10, 12, 20, 21, 22];

/// The Minesweeper count written as the tables-by-tens tip's "By tens, one number" is: the board laid out by tens
/// by the program itself, square (r, c) at 11 + 10r + c inside a border of empty squares, so the square read as one
/// number s (20 for row 2, column 0) has its top-left neighbour at s, and the rest at the offsets from it. The
/// input is promised to be a square on the board.
pub fn main() {
    #[rustfmt::skip]
    let b: [i16; 58] = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 1, 0, 0, 0, 1, 0, 0, 0,
        0, 0, 0, 0, 1, 0, 0, 0, 0, 0,
        0, 0, 0, 1, 0, 0, 0, 0, 0, 0,
        0, 1, 0, 0, 0, 1, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0,
    ];
    let s = inp() as usize;
    unsafe { assume(s < 36) };
    unsafe { assume(s % 10 < 6) };
    let mut n = 0;
    for off in OFFSETS {
        n += b[s + off];
    }
    out(n);
}
