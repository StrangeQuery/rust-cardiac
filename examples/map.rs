//! MapReduce's map: one machine's share of the counting. The deck is a shard of a longer text, a word to a card
//! (words are numbers, 1 to 16, as `rot13`'s letters are; anything else is skipped), ended with -999. INP waits
//! for each card, but at the end of the input it stops the machine where it stands, with no chance to output
//! anything: so the shard says when it is over. It outputs the shard's counts, `word, count` for each word it saw,
//! and -999: a deck for the reducer (`reduce`).
//!
//! Each machine counts its own shard, so the words are added up where they are read and the deck carries a count
//! for each word, not a card for each time it was read: the paper's combiner, here because a deck is cards.
use cardiac::*;

const WORDS: usize = 16;

fn main() {
    let mut count = [0; WORDS];
    loop {
        let w = inp();
        if w == -999 {
            break;
        }
        if (1..=WORDS as i16).contains(&w) {
            count[w as usize - 1] += 1;
        }
    }
    for i in 0..WORDS {
        if count[i] > 0 {
            out(i as i16 + 1);
            out(count[i]);
        }
    }
    out(-999);
}
