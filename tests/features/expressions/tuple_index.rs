//! feature: A tuple and `.0`, `.1`
//! section: Tuple and tuple indexing expressions
//! input: 3 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let t = (inp(), inp());
    out(t.1);
    out(t.0 + t.1);
}
