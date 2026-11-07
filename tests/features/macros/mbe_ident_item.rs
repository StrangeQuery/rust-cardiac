//! feature: a macro that defines a function from an `$name:ident`
//! section: Macros by example
//! input: 7
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
macro_rules! adder {
    ($name:ident, $k:literal) => {
        fn $name(x: i16) -> i16 {
            x + $k
        }
    };
}
adder!(add_five, 5);
adder!(add_hundred, 100);
pub fn main() {
    let a = inp();
    out(add_five(a));
    out(add_hundred(a));
}
