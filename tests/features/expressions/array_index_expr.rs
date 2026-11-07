//! feature: `t[i + 1]` and `t[n - i]`: an index computed in place
//! section: Array and array index expressions
//! input: 1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let t = [10, 20, 30, 40, 50];
    let i = inp() as usize;
    out(t[i + 1]);
    out(t[4 - i]);
    out(t[i * 2]);
}
