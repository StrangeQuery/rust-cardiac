//! feature: An array literal `[a, b, c]` with values read at run time
//! section: Array and array index expressions
//! input: 4 5 6 2
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let t = [inp(), inp(), inp()];
    let i = inp() as usize;
    out(t[i]);
    out(t[0] + t[1]);
}
