#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// Loops that add what doesn't change a constant number of times, whose totals are worked out at once: from 0 as
/// a product, and from a value as it plus a product.
#[inline(never)]
fn twelve(x: i16) -> i16 {
    let mut t = 0;
    for _ in 0..12 {
        t += x;
    }
    t
}

#[inline(never)]
fn seven_less(t: i16, d: i16) -> i16 {
    let d = d % 10;
    let mut t = t % 100;
    for _ in 0..7 {
        t -= d;
    }
    t
}

pub fn main() {
    for x in [-83, 5, 83] {
        out(twelve(x));
        out(seven_less(x, x));
    }
}
