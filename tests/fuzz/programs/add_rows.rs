//! Loops that add what doesn't change a small run-time number of times, which the compiler makes jumps into rows of
//! ADDs (addrow.rs): a count from `% 10`, adding a value twice a pass, and a promised digit, taking one away. A
//! total past a card overflows.
use cardiac::*;

#[inline(never)]
fn promised(d: i16, x: i16, t: i16) -> i16 {
    // SAFETY: the fuzzer's first input is 0..9
    unsafe { assume(d >= 0 && d <= 9) };
    let mut t = t;
    for _ in 0..d {
        t -= x;
    }
    t
}

fn main() {
    loop {
        let d = inp();
        let x = inp();
        let t = inp();
        let mut a = t;
        for _ in 0..x % 10 {
            a += d + d;
        }
        out(a);
        out(promised(d, x, t));
    }
}
