#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

struct Hand {
    cards: [i16; 3],
    n: i16,
}

#[inline(never)]
fn deal(v: i16) -> Hand {
    Hand { cards: [v, 4, -v], n: 3 }
}

#[inline(never)]
fn show(h: Hand) {
    out(h.cards[0] + h.n);
}

/// A struct with an array field: the array is one of its cells, returned (every element) and passed.
pub fn main() {
    let h = deal(5);
    out(h.cards[2]);
    show(h);
}
