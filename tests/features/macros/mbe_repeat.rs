//! feature: `macro_rules!` with a repetition `$($e:expr),*`
//! section: Macros by example
//! input: 7 3 -2
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
macro_rules! sum {
    ($($e:expr),*) => {
        0 $(+ $e)*
    };
}
pub fn main() {
    let a = inp();
    let b = inp();
    let c = inp();
    out(sum!(a, b, c));
    out(sum!(a));
    out(sum!());
}
