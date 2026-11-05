//! MapReduce's reduce: the map machines' decks (`map`), stacked, added up. The first card says how many decks are
//! behind it; each deck is `word, count` cards ended with -999. It outputs `word, total` for each word, in order.
use cardiac::*;

const WORDS: usize = 16;

fn main() {
    let mut total = [0; WORDS];
    let mut decks = inp();
    while decks > 0 {
        loop {
            let w = inp();
            if w == -999 {
                break;
            }
            let n = inp();
            if (1..=WORDS as i16).contains(&w) {
                total[w as usize - 1] += n;
            }
        }
        decks -= 1;
    }
    for i in 0..WORDS {
        if total[i] > 0 {
            out(i as i16 + 1);
            out(total[i]);
        }
    }
}
