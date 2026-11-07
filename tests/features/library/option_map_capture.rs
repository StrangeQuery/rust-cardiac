//! feature: `Option::map` with a closure that captures a variable
//! section: Option
//! disposition: demand
//! note: The closure captures a variable, which the compiler can't yet pass where rustc calls `map` (opt-level 1).
//! input: 10 5 -1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn maybe(a: i16) -> Option<i16> {
    if a > 0 { Some(a) } else { None }
}
pub fn main() {
    let c = inp();
    for _ in 0..2 {
        let x = inp();
        let o = maybe(x);
        out(o.map(|v| v + c).unwrap_or(0));
    }
}
