//! feature: `&x` and `&(a, b)` in a `match`
//! section: Reference patterns
//! input: 6 2
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let t = (inp(), inp());
    let r = &t;
    match r {
        &(a, b) => out(a * b),
    }
    let v = &t.0;
    let &w = v;
    out(w);
}
