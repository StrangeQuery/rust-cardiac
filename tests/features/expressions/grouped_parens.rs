//! feature: Parentheses changing the order: `(a + b) * 3` against `a + b * 3`
//! section: Grouped expressions
//! input: 4 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    let b = inp();
    out((a + b) * 3);
    out(a + b * 3);
    out(a - (b - 1));
}
