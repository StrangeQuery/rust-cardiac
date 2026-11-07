//! feature: a recursive macro (the largest of its arguments)
//! section: Macros by example
//! input: 7 30 -3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
macro_rules! biggest {
    ($x:expr) => { $x };
    ($x:expr, $($rest:expr),+) => {{
        let r = biggest!($($rest),+);
        if $x > r { $x } else { r }
    }};
}
pub fn main() {
    let a = inp();
    let b = inp();
    let c = inp();
    out(biggest!(a, b, c));
}
