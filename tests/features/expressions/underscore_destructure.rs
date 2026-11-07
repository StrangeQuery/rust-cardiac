//! feature: `_` in destructuring assignment, `(a, _) = (x, y)`
//! section: Underscore expressions
//! input: 3 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let mut a = 0;
    let b;
    (a, _) = (inp(), inp());
    (_, b) = (a, a + 1);
    out(a);
    out(b);
}
