#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// Without overflow checks a result past a card is what its cell keeps, the last three digits with the sign: 30 *
/// 40 is 1200, kept as 200. Each value is printed and then compared, and the comparison must see what was printed:
/// the accumulator still holds the whole result after the store, so it can't stand in for the cell.
#[inline(never)]
fn product(a: i16) {
    let p = a.wrapping_mul(40);
    out(p);
    out(if p > 500 { 1 } else { 0 });
}

#[inline(never)]
fn sum(a: i16, b: i16) {
    let s = a.wrapping_add(b);
    out(s);
    out(if s > 500 { 1 } else { 0 });
    let d = (-a).wrapping_sub(b);
    out(d);
    out(if d < -500 { 1 } else { 0 });
}

pub fn main() {
    product(30);
    product(10);
    sum(600, 500);
    sum(200, 100);
}
