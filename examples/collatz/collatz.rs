//! The Collatz sequence: from a number read off a card, halve it if it is even, else take 3n + 1, until it
//! reaches 1. Try 7, or 27, which climbs past 999: a card can't hold that, so both builds print -999 and stop
//! (this package's profile turns on overflow checks, so the 3n + 1 that overflows panics).
use cardiac::*;

fn main() {
    let mut n = inp();
    out(n);
    while n > 1 {
        n = if n % 2 == 0 { n / 2 } else { 3 * n + 1 };
        out(n);
    }
}
