#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// Without overflow checks a result past a card is what its cell keeps, the last three digits with the sign: 30 *
/// 40 is 1200, kept as 200, which is what is printed. The comparison that comes straight after reads the
/// accumulator, which still holds the whole result, so it sees 1200, as native Rust does.
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
