//! Signed division and remainder of two numbers off cards, every sign: the division routine, its zero check, and
//! the remainder rustc works out from the same check. A divisor of 0 panics both ways.
use cardiac::*;

fn main() {
    let x = inp();
    let y = inp();
    out(x / y);
    out(x % y);
}
