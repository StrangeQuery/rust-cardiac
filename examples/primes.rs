//! The primes below 50, by the sieve of Eratosthenes: cross out every multiple of each number, and what is left
//! is prime.
use cardiac::*;

const N: usize = 50;

fn main() {
    let mut crossed = [false; N];
    for p in 2..N {
        if !crossed[p] {
            out(p as i16);
            let mut m = p + p;
            while m < N {
                crossed[m] = true;
                m += p;
            }
        }
    }
}
