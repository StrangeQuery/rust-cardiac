#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// Comparisons with a constant just past what a card holds (1000, -1000) on the left, `1000 > x`, against values
/// at the card's ends and between: each is the test with the card's last value (`1000 > x` is `999 >= x`), so the
/// constant never needs a cell. Prints 1 or 0 for each. compare_past.rs has the constant on the right.
#[inline(never)]
fn tests(x: i16) {
    out(if 1000 > x { 1 } else { 0 });
    out(if 1000 <= x { 1 } else { 0 });
    out(if -1000 < x { 1 } else { 0 });
    out(if -1000 >= x { 1 } else { 0 });
}

pub fn main() {
    let values: [i16; 3] = [-999, 0, 999];
    let mut k: usize = 0;
    while k < 3 {
        tests(values[k]);
        k += 1;
    }
}
