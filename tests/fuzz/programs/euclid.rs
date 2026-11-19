//! `rem_euclid` by a variable: the divide routine's remainder, made positive. A divisor of 0 panics both ways.
use cardiac::*;

fn main() {
    let x = inp();
    let y = inp();
    out(x.rem_euclid(y));
}
