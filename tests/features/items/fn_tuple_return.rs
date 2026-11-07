//! feature: Several results returned as a tuple
//! section: Functions
//! input: 17 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn divmod(a: i16, b: i16) -> (i16, i16) { (a / b, a % b) }
pub fn main() {
    let a = inp();
    let b = inp();
    let (q, r) = divmod(a, b);
    out(q);
    out(r);
}
