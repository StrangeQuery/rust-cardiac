//! An array of arrays, laid out by tens: read at a row and column from the input and a constant away from them,
//! with and without wrapping. An index off the grid, or past a row's end, panics both ways.
use cardiac::*;

fn main() {
    let t = [[1i16, 2, 3, 4], [5, 6, 7, 8], [9, 10, 11, 12]];
    let r = inp() as usize;
    let c = inp() as usize;
    out(t[r + 1][c + 2]);
    out(t[r.wrapping_add(1)][c]);
}
