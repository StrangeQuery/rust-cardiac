//! feature: `macro_rules!` with one rule and an `$e:expr`
//! section: Macros by example
//! input: 7
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
macro_rules! twice {
    ($e:expr) => {
        $e + $e
    };
}
pub fn main() {
    let a = inp();
    out(twice!(a));
}
