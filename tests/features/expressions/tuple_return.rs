//! feature: A tuple returned from a function and taken apart
//! section: Tuple and tuple indexing expressions
//! input: 17 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn sumdiff(a: i16, b: i16) -> (i16, i16) { (a + b, a - b) }
pub fn main() {
    let a = inp();
    let b = inp();
    let (s, d) = sumdiff(a, b);
    out(s);
    out(d);
    let t = sumdiff(a, 2);
    out(t.0 + t.1);
}
