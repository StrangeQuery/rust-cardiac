#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

#[derive(Clone, Copy)]
struct Card {
    n: i16,
    kind: Option<i16>,
}

#[inline(never)]
fn mark(c: &mut Card) {
    if c.kind.is_none() {
        c.kind = Some(c.n);
    }
}

/// A struct with an enum in it, in an array (one array per cell), built from a constant variant
/// (`kind: None`), and behind a `&mut`.
pub fn main() {
    let mut cs = [Card { n: 4, kind: None }; 2];
    let mut c = cs[1];
    mark(&mut c);
    cs[1] = c;
    out(cs[0].kind.unwrap_or(-1));
    out(cs[1].kind.unwrap_or(-1));
}
