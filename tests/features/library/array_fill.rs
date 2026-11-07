//! feature: `array.fill(x)`
//! section: Arrays and slices
//! disposition: easy
//! note: On an array of known length it is a short loop the compiler already writes for `for i in 0..N`.
//! input: 7
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let mut t: [i16; 4] = [0; 4];
    t.fill(inp());
    for i in 0..t.len() {
        out(t[i]);
    }
}
