#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// `checked_add` and `checked_sub`, which return an Option: at opt-level 1 and s rustc calls them, and they
/// are what it inlines at 2. Within a card here, so each is `Some`, as natively (checked_card.rs has the `None`s,
/// past a card; checked_mul.rs `checked_mul`).
#[inline(never)]
fn step(v: i16, d: i16) -> i16 {
    match v.checked_add(d) {
        Some(x) => x,
        None => -1,
    }
}

pub fn main() {
    let mut v: i16 = -30;
    while v < 30 {
        out(step(v, 7));
        out(v.checked_sub(12).unwrap_or(-500));
        v += 11;
    }
}
