//! feature: `Option::zip`
//! section: Option
//! disposition: demand
//! note: The payload of the result is a tuple, and an enum's fields must be scalars for now.
//! input: 1 2 1 -2 -1 2 -1 -2
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
    for _ in 0..4 {
        let a = inp();
        let b = inp();
        let o = maybe(a);
        let p = maybe(b);
        out(match o.zip(p) { Some((x, y)) => x * 10 + y, None => -1 });
    }
}
