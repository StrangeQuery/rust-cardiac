//! feature: A closure passed to a generic `F: Fn(i16) -> i16` parameter
//! section: Closure types
//! disposition: demand
//! note: A generic function rustc doesn't inline gets one copy per closure type, each an ordinary function.
//! input: 5 3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn apply<F: Fn(i16) -> i16>(f: F, x: i16) -> i16 {
    f(x)
}
pub fn main() {
    let k = inp();
    out(apply(|x| x + k, inp()));
}
