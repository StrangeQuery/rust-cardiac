//! feature: `Option::get_or_insert`
//! section: Option
//! disposition: demand
//! note: It returns a `&mut` into the `Option`, and at opt-level 2 it goes through a closure inside core.
//! input: 8 5 -1
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
        let mut o = maybe(x);
        let r = o.get_or_insert(c);
        *r += 1;
        out(o.unwrap_or(0));
    }
}
