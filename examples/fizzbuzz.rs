//! FizzBuzz from 1 to 100. CARDIAC prints only numbers, so the words are negative numbers: a multiple of 3
//! (Fizz) prints -3, a multiple of 5 (Buzz) prints -5, and a multiple of both (FizzBuzz) prints -15.
use cardiac::*;

fn main() {
    for n in 1..=100 {
        if n % 15 == 0 {
            out(-15);
        } else if n % 3 == 0 {
            out(-3);
        } else if n % 5 == 0 {
            out(-5);
        } else {
            out(n);
        }
    }
}
