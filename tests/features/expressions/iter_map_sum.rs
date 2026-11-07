//! feature: `array.iter().map(...).sum()`
//! section: Loop expressions
//! disposition: demand
//! input: 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    let t: [i16; 3] = [1, 2, a];
    let s: i16 = t.iter().map(|x| x * 2).sum();
    out(s);
}
