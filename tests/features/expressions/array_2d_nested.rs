//! feature: An array of arrays, `t[r][c]`
//! section: Array and array index expressions
//! disposition: demand
//! note: One flat array indexed `r * W + c` already works; an array of arrays is the same thing with the arithmetic written by the compiler.
//! input: 1 2
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let mut t = [[0i16; 3]; 3];
    let r = inp() as usize;
    let c = inp() as usize;
    t[r][c] = 5;
    t[c][r] += 7;
    out(t[r][c]);
    out(t[c][r]);
}
