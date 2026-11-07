//! feature: `macro_rules!` with several rules, chosen by the tokens
//! section: Macros by example
//! input: 7 3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
macro_rules! pick {
    (first $a:expr, $b:expr) => {
        $a
    };
    (second $a:expr, $b:expr) => {
        $b
    };
}
pub fn main() {
    let a = inp();
    let b = inp();
    out(pick!(first a, b));
    out(pick!(second a, b));
}
