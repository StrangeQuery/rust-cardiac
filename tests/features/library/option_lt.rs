//! feature: `<` between two `Option<i16>` (`None` is least)
//! section: Option
//! disposition: demand
//! note: It goes through `Ordering` (an `i8`) at opt-level 2 and a `PartialOrd::lt` call at opt-level 1.
//! input: 1 2 2 1 1 -2 -1 2
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
        out((o < p) as i16);
    }
}
