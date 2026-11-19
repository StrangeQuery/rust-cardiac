//! A digit times c, over 10, as a match whose arms each add c once more than the one before (#35): the arms share
//! their ends, and where the program needs the cells, one row of ADDs.
use cardiac::*;

fn main() {
    let d = inp();
    let c = inp();
    // SAFETY: the fuzzer gives a digit
    unsafe { assume(d >= 0 && d <= 9) };
    let s = match d {
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
    };
    out(s);
    out(c - s);
}
