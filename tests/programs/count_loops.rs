#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// Loops whose counter only counts (`for _ in a..b`, a hand-written `while`) count to 0, CARDIAC's one test: each
/// must run as often as before, inclusive, reversed, empty, and beside a counter that is read.
pub fn main() {
    let mut total = 0;
    for _ in 0..7 {
        total += 1;
    }
    for _ in 3..=5 {
        total += 10;
    }
    for _ in (2..4).rev() {
        total += 100;
    }
    out(total);
    let mut k = 0;
    while k < 4 {
        total -= 1;
        k += 1;
    }
    out(total);
    for _ in 5..5 {
        out(-1);
    }
    for i in -3..2 {
        out(i);
    }
}
