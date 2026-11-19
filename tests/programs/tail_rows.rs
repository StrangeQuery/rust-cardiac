#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

unsafe fn assume(condition: bool) {
    unsafe { core::hint::assert_unchecked(condition) }
}

/// The arms of a match that end alike (#35): arm k is the last k - 1 ADDs of arm 9, its shift and its store, so the
/// arms share their shift and store with the arm that runs on into what follows, for no instruction more. Each arm
/// still has its own ADDs; they are one row only where the program needs the cells (tail_rows2).
#[inline(never)]
fn tenth(d: i16, c: i16) -> i16 {
    // SAFETY: main passes digits
    unsafe { assume(d >= 0 && d <= 9) };
    match d {
        0 => 0,
        1 => c / 10,
        2 => (c + c) / 10,
        3 => (c + c + c) / 10,
        4 => (c + c + c + c) / 10,
        5 => (c + c + c + c + c) / 10,
        6 => (c + c + c + c + c + c) / 10,
        7 => (c + c + c + c + c + c + c) / 10,
        8 => (c + c + c + c + c + c + c + c) / 10,
        _ => (c + c + c + c + c + c + c + c + c) / 10,
    }
}

const DS: [i16; 6] = [0, 1, 4, 8, 9, 9];
const CS: [i16; 6] = [7, -5, -21, 11, 99, -99];

pub fn main() {
    for i in 0..6 {
        out(tenth(DS[i], CS[i]));
    }
}
