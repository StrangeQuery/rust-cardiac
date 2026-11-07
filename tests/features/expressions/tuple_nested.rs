//! feature: A nested tuple, `t.1.0`
//! section: Tuple and tuple indexing expressions
//! input: 3 4 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let mut t = (inp(), (inp(), inp()));
    t.1.0 += 10;
    out(t.0);
    out(t.1.0);
    out(t.1.1);
}
