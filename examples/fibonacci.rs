//! The Fibonacci numbers, up to the last one a card can hold: 0, 1, 1, 2, 3, 5, ... 987.
use cardiac::*;

fn main() {
    let (mut a, mut b) = (0, 1);
    out(a);
    loop {
        out(b);
        if a > 999 - b {
            break; // the next one, a + b, is more than a card holds
        }
        (a, b) = (b, a + b);
    }
}
