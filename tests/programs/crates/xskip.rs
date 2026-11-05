//! skip_cards() from the built-ins crate, on a deck: natively a loop of inp(), on CARDIAC a computed jump
//! into a row of INP 00, run once more for each ten. Deck: how many groups, then for each a count n (0 to 999), n
//! cards to drop, a count of more to drop, those, and a card to print; then nine cards to drop and one to print.
use cardiac::*;

fn main() {
    let groups = inp();
    for _ in 0..groups {
        let n = inp();
        skip_cards(n);
        let more = inp();
        skip_cards(more);
        out(inp());
    }
    // a few in place, more by the row
    skip_cards(2);
    skip_cards(7);
    out(inp());
}
