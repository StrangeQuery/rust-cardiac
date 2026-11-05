//! Guess the number: CARDIAC thinks of a number from 1 to 100 and you guess it. Each guess is answered with a card:
//! 1 to go higher, -1 to go lower. Guess it and CARDIAC outputs 0, then how many guesses it took, and stops.
//!
//! CARDIAC has no clock to seed a random number from, so the first card is the seed: any number from 0 to 999.
//! Typing the guesses as it asks, `burn guess.cqb`; or all at once,
//! `burn guess.cqb -- 123 50 75 63 69 66`.
use cardiac::*;

/// The next random number after s, both 0 to 999: s -> (21 s + 17) mod 1000, which visits all 1000 before it
/// repeats (a larger program's generator). Worked out so that no step leaves three digits: 21 s is 20 s + s, and 20 s mod 1000
/// is (2 s mod 100) x 10.
fn next(s: i16) -> i16 {
    let a = s * 2 % 100 * 10;
    let room = 982 - s;
    if a > room {
        a - room - 1
    } else {
        a + s + 17
    }
}

fn main() {
    // SAFETY: the seed is a card from 0 to 999, as the deck says
    let seed = inp();
    unsafe { assume((0..=999).contains(&seed)) };
    let secret = next(seed) / 10 + 1;
    let mut guesses = 0;
    loop {
        let guess = inp();
        guesses += 1;
        if guess < secret {
            out(1);
        } else if guess > secret {
            out(-1);
        } else {
            break;
        }
    }
    out(0);
    out(guesses);
}
