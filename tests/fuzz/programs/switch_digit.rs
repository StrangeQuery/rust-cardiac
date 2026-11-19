//! A dense match through a jump table (#34), on a digit promised 0 to 9: no test of its range.
use cardiac::*;

fn main() {
    let d = inp();
    let c = inp();
    // SAFETY: the fuzzer gives d from 0 to 9
    unsafe { assume(d >= 0 && d <= 9) };
    out(match d {
        0 => 0,
        1 => c,
        2 => c + c,
        3 => -c,
        4 => c - 1,
        5 => c + 1,
        6 => 1 - c,
        7 => c + 2,
        8 => -c - c,
        _ => c - 2,
    });
}
