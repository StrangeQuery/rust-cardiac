//! feature: Array pattern through `&[i16; 3]`
//! section: Slice patterns
//! input: 4 5 6
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn mid(t: &[i16; 3]) -> i16 {
    let [_, m, _] = t;
    *m
}
pub fn main() {
    let t = [inp(), inp(), inp()];
    out(mid(&t));
}
