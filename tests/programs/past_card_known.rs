#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// Without overflow checks a product past a card is what its cell keeps: a from 25 to 30 makes a * 40 1000 to 1200,
/// kept as 0 to 200. The range pass knows the cell's value from that, not the product's, and decides no test from
/// it: 200 is not more than 500.
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
