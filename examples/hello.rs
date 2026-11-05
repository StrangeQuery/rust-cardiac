//! Hello, world. CARDIAC's teletype prints numbers, so this one counts down: 10, 9, ... 1, 0.
use cardiac::*;

fn main() {
    for n in (0..=10).rev() {
        out(n);
    }
}
