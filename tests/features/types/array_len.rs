//! feature: `.len()` of an array passed by reference
//! section: Array types
//! input: 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn last(t: &[i16; 5]) -> i16 {
    t[t.len() - 1]
}
pub fn main() {
    let a = inp();
    let t = [a, a + 1, a + 2, a + 3, a + 4];
    out(last(&t));
    out(t.len() as i16);
}
