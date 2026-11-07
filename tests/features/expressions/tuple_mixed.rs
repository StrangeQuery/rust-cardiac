//! feature: A tuple of `i16`, `bool` and `u8`
//! section: Tuple and tuple indexing expressions
//! input: 3 1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn mk(a: i16, b: i16) -> (i16, bool, u8) { (a * 2, b != 0, if b != 0 { 7 } else { 9 }) }
pub fn main() {
    let a = inp();
    let b = inp();
    let t = mk(a, b);
    out(t.0);
    out(t.1 as i16);
    out(t.2 as i16);
}
