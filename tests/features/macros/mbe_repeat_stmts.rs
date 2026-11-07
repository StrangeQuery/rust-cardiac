//! feature: a repetition expanding to one statement each
//! section: Macros by example
//! input: 7 3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
macro_rules! out_all {
    ($($e:expr),+ $(,)?) => {
        $(out($e);)+
    };
}
pub fn main() {
    let a = inp();
    let b = inp();
    out_all!(a, b, a + b,);
}
