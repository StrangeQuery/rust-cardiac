//! ROT13: each letter moved 13 places along the alphabet, so doing it twice gives the message back. CARDIAC has no
//! letters, so a letter is its place, A = 1 to Z = 26; anything else (0 for a space, say) goes through as it is.
//!
//! The message is the deck behind the program, as it was on CARDIAC: the program reads a card, outputs its
//! rotation, and reads the next, until the reader is empty. Two in a pipe give the message back:
//! `burn rot13.cqb -- 8 5 12 12 15 | burn rot13.cqb`.
use cardiac::*;

fn main() {
    loop {
        let c = inp();
        let r = if (1..=13).contains(&c) {
            c + 13
        } else if (14..=26).contains(&c) {
            c - 13
        } else {
            c
        };
        out(r);
    }
}
