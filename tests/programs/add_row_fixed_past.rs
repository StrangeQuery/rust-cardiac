#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// A total added a constant number of times past a card: with overflow checks CARDIAC stops where the loop's sum
/// leaves a card; without them the cell keeps what the loop's stores kept, the product's last three digits.
#[inline(never)]
fn twelve(x: i16) -> i16 {
    let mut t = 0;
    for _ in 0..12 {
        t += x;
    }
    t
}

pub fn main() {
    out(twelve(83));
    out(twelve(100));
    out(twelve(-84));
}
