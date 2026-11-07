//! feature: Match ergonomics: `&(i16, i16)` matched with `(a, b)`
//! section: Binding modes
//! disposition: easy
//! input: 6 2 -3 9
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn diff(t: &(i16, i16)) -> i16 {
    let (a, b) = t;
    *a - *b
}
pub fn main() {
    for _ in 0..2 {
        let t = (inp(), inp());
        out(diff(&t));
    }
}
