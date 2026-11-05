#![no_std]
#[inline(never)]
pub fn inp() -> i16 {
    0
}
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn skip_cards(n: i16) {
    let mut k = 0;
    while k < n {
        inp();
        k += 1;
    }
}

/// One skip_cards(n), n from the deck: test_compiler.sh checks the card after the n dropped, and the
/// instructions the run takes for each n (the routine's 15, one a card, and two more for each ten).
pub fn main() {
    let n = inp();
    skip_cards(n);
    out(inp());
}
