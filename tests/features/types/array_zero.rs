//! feature: A zero-length array `[i16; 0]`
//! section: Array types
//! input: 3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn total(t: &[i16; 0]) -> i16 {
    let mut s = 0;
    let mut i = 0;
    while i < t.len() {
        s += t[i];
        i += 1;
    }
    s
}
pub fn main() {
    let t: [i16; 0] = [];
    out(total(&t) + inp());
    out(t.len() as i16);
}
