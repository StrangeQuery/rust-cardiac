//! feature: a macro that expands to statements
//! section: Macros by example
//! input: 7 3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
macro_rules! show_both {
    ($a:ident, $b:ident) => {
        out($a);
        out($b);
    };
}
pub fn main() {
    let a = inp();
    let b = inp();
    show_both!(a, b);
}
