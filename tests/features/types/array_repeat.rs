//! feature: An array made with `[x; N]` from an input
//! section: Array types
//! input: 9 2
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let x = inp();
    let t = [x; 4];
    let i = inp() as usize;
    out(t[i]);
    out(t[0] + t[3]);
}
