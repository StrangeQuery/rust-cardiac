#[inline(never)]
pub fn out(_v: i16) {}

use kit::tables::{square, SQUARES};

/// A const table from another crate, read here, and the same table read by a function there.
pub fn main() {
    let squares = SQUARES;
    let mut i = 0;
    while i < 5 {
        out(squares[i] - square(4 - i));
        i += 1;
    }
}
