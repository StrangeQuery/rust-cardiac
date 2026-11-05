#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

struct Card {
    n: i16,
    kind: Option<i16>,
}

#[inline(never)]
fn make(n: i16) -> Card {
    Card {
        n,
        kind: if n > 2 { Some(n * 10) } else { None },
    }
}

#[inline(never)]
fn show(c: Card) {
    out(c.n);
    out(c.kind.unwrap_or(-1));
}

/// An enum with data as a struct's field: the struct's cells are its own and the enum's, built, returned,
/// passed and matched.
pub fn main() {
    let mut i = 0;
    while i < 5 {
        let c = make(i);
        if let Some(k) = c.kind {
            out(k + 1);
        }
        show(c);
        i += 1;
    }
}
