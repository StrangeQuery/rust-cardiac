#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

unsafe fn assume(condition: bool) {
    unsafe { core::hint::assert_unchecked(condition) }
}

/// A digit times a value, as a match whose arms each add c once more than the one before (#35), and a little more
/// work after it. Too big with a copy of each arm's adds, it fits once the arms share one row of ADDs, each arm jumping
/// in at its own cell.
#[inline(never)]
fn times(d: i16, c: i16) -> i16 {
    // SAFETY: main passes digits
    unsafe { assume(d >= 0 && d <= 9) };
    let s = match d {
        0 => 0,
        1 => c,
        2 => c + c,
        3 => c + c + c,
        4 => c + c + c + c,
        5 => c + c + c + c + c,
        6 => c + c + c + c + c + c,
        7 => c + c + c + c + c + c + c,
        8 => c + c + c + c + c + c + c + c,
        _ => c + c + c + c + c + c + c + c + c,
    };
    out(d - s);
    s
}

const DS: [i16; 6] = [0, 1, 4, 8, 9, 9];
const CS: [i16; 6] = [7, -5, -21, 11, 99, -99];

pub fn main() {
    for i in 0..6 {
        out(times(DS[i], CS[i]));
    }
}
