//! feature: `Option::map` with a closure that captures nothing
//! section: Option
//! input: 5 -1
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
    for _ in 0..2 {
        let a = inp();
        let o = maybe(a);
        out(o.map(|x| x + 1).unwrap_or(0));
    }
}
