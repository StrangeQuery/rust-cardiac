//! 2 to the power n, for n read off a card (up to 99), digit by digit. 2^99 has 30 digits, far too many for one
//! card, so the number is kept as an array of digits, least significant first, and doubled the way it is done by
//! hand: each digit twice over, plus the carry. Prints the digits most significant first: for 20, 1 0 4 8 5 7 6.
//!
//! The program is bigger than CARDIAC's memory, so it comes in two phases. `overlay()` ends the first: the
//! second phase's code is loaded over the first's, and the digits, which are data, stay where they are.
use cardiac::*;

const DIGITS: usize = 30;

fn main() {
    let n = inp();
    let mut d = [0; DIGITS];
    d[0] = 1;
    for _ in 0..n {
        let mut carry = 0;
        // an index loop: CARDIAC programs index arrays by a range (iterators over arrays are not supported)
        #[allow(clippy::needless_range_loop)]
        for i in 0..DIGITS {
            let x = d[i] + d[i] + carry;
            (d[i], carry) = if x >= 10 { (x - 10, 1) } else { (x, 0) };
        }
    }
    overlay();
    // the leading zeros aren't printed
    let mut top = DIGITS - 1;
    while top > 0 && d[top] == 0 {
        top -= 1;
    }
    for i in (0..=top).rev() {
        out(d[i]);
    }
}
