//! The greatest common divisor of two numbers, by Euclid's algorithm: reads the two numbers, prints the largest
//! number that divides both.
//!
//!     cargo run --bin gcd <<< "252 105"
use cardiac::*;

fn main() {
    let mut a = inp();
    let mut b = inp();
    while b != 0 {
        (a, b) = (b, a % b);
    }
    out(a);
}
