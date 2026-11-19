#![no_std]
#[inline(never)]
pub fn inp() -> i16 {
    0
}
#[inline(never)]
pub fn out(_v: i16) {}

/// a * b of two cards from the deck, by the multiply routine digit by digit: test_compiler.sh checks the product
/// and the instructions the run takes, 14 for each digit of b and one for each addition.
pub fn main() {
    let a = inp();
    let b = inp();
    out(a * b);
}
