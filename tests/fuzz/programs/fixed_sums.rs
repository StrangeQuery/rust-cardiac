//! Loops that add what doesn't change a constant number of times, which the compiler works out at once (addrow.rs):
//! from 0, the product; from a value, it plus the product; a constant step; a total past a card overflows.
use cardiac::*;

#[inline(never)]
fn from(t: i16, d: i16) -> i16 {
    let mut t = t;
    for _ in 0..9 {
        t -= d;
    }
    t
}

fn main() {
    loop {
        let x = inp();
        let y = inp();
        let mut t = 0;
        let mut k = 3;
        for _ in 0..12 {
            t += x;
            k += 2;
        }
        out(t);
        out(k);
        out(from(y, x % 10));
    }
}
