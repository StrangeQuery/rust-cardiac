#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// Without overflow checks a product past a card is what its cell keeps: a from 25 to 30 makes a * 40 1000 to 1200,
/// kept as 0 to 200. The test straight after reads the accumulator, which holds all of it, as native Rust's does;
/// the range pass allows for either, the product or what the cell keeps, and decides nothing from it.
#[inline(never)]
fn known(a: i16) {
    if a >= 25 && a <= 30 {
        let p = a * 40;
        out(p);
        if p > 500 {
            out(1);
        } else {
            out(0);
        }
    }
}

pub fn main() {
    known(30);
    known(26);
}
