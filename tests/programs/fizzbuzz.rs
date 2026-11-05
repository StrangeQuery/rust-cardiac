#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

// CARDIAC prints numbers, so the words are negative numbers: -3 Fizz, -5 Buzz, -15 FizzBuzz.
const FIZZ: i16 = -3;
const BUZZ: i16 = -5;
const FIZZBUZZ: i16 = -15;

pub fn main() {
    let mut i: i16 = 1;
    while i <= 100 {
        if i % 15 == 0 {
            out(FIZZBUZZ);
        } else if i % 3 == 0 {
            out(FIZZ);
        } else if i % 5 == 0 {
            out(BUZZ);
        } else {
            out(i);
        }
        i += 1;
    }
}
