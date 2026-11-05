#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// The next card's column and value, while cards are left (a larger program's stamp machine reads ahead so).
#[inline(never)]
fn next_card(n: i16, at: i16) -> Option<(i16, i16)> {
    if n <= 0 {
        None
    } else {
        Some((at * 2, at - 5))
    }
}

/// An Option of a tuple: the tuple is the struct of its fields a tuple crossing a call already is, here one
/// of the Option's cells.
pub fn main() {
    let mut n = 3;
    let mut at = 0;
    while let Some((column, value)) = next_card(n, at) {
        out(column);
        out(value);
        n -= 1;
        at += 1;
    }
    out(-1);
}
