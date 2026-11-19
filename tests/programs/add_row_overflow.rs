#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// A total made a row (addrow.rs) that passes a card: with overflow checks CARDIAC stops where the loop would have,
/// at its last addition (natively an i16 goes on to 32767); without them the cell keeps the last three digits, as
/// the loop's stores did.
#[inline(never)]
fn total(d: i16, x: i16, t: i16) -> i16 {
    let mut t = t;
    for _ in 0..d % 10 {
        t += x;
    }
    t
}

pub fn main() {
    let mut d = 0;
    while d < 10 {
        out(total(d, 125, 0));
        out(total(d, -125, 5));
        d += 1;
    }
}
