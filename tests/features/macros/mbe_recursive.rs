//! feature: a recursive macro (counting its arguments)
//! section: Macros by example
//! input: 7
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
macro_rules! count {
    () => { 0 };
    ($x:tt $($rest:tt)*) => { 1 + count!($($rest)*) };
}
pub fn main() {
    let a = inp();
    out(a + count!(a b c d));
}
