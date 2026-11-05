#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

const T: [i16; 4] = [10, 11, 12, 13];

/// Without overflow checks 0 - 1 as usize is usize::MAX, a usize past a card, held as -1: it indexes nothing,
/// and the bounds check panics, as in Rust (with overflow checks, the - does). As -1 it passed the check, and read
/// the cell before the table.
#[inline(never)]
fn below(i: usize) {
    let j = i - 1;
    out(j as i16);
    out(T[j]);
}

pub fn main() {
    below(2);
    below(0);
}
