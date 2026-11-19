#![no_std]
#[inline(never)]
pub fn inp() -> i16 {
    0
}
#[inline(never)]
pub fn out(_v: i16) {}

/// a / b and a % b of two cards from the deck, by the divide routine digit by digit: test_compiler.sh checks the
/// quotient, the remainder and the instructions the run takes, about 8 for each unit of each digit of the quotient.
pub fn main() {
    let a = inp();
    let b = inp();
    out(a / b);
    out(a % b);
}
