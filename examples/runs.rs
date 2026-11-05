//! Runs of positive cards, with flags. Reads cards until -999; outputs each card where the sign changes after a
//! zero has been seen, then how many runs of positives there were, then 0 if any card was a zero.
//!
//! The three flags are `cardiac::Flag`s, not `bool`s: kept in cells and tested again and again, each test is one
//! instruction less on CARDIAC (false is -1, so the test is its sign). The trues and falses are the same.
use cardiac::*;

fn main() {
    let mut in_run: Flag = false.into();
    let mut seen_zero: Flag = false.into();
    let mut last_up: Flag = false.into();
    let mut runs = 0;
    loop {
        let v = inp();
        if v == -999 {
            break;
        }
        let up: Flag = (v > 0).into();
        if v == 0 {
            seen_zero = true.into();
        }
        if up.get() && !in_run.get() {
            runs += 1;
        }
        if up != last_up && seen_zero.get() {
            out(v);
        }
        in_run = up;
        last_up = up;
    }
    out(runs);
    if seen_zero.get() {
        out(0);
    }
}
