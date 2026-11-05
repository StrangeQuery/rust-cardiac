#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
/// A value rustc can't see through, as a card read would be.
#[inline(never)]
pub fn card(v: i16) -> i16 {
    v
}

/// A bool from `a && b`, tested at once and again later: rustc's join of its two ways goes, and the bool's
/// test is the second's compare, read again later; true, it must still be stored.
pub fn main() {
    let start = card(10);
    let past = card(49);
    let end_wall = start > 0 && past >= 0;
    if (end_wall && past < 120) || past < -5 {
        out(if end_wall { past } else { -1 });
    }
}
