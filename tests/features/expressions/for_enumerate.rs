//! feature: `for (i, x) in array.iter().enumerate()`
//! section: Loop expressions
//! disposition: easy
//! input: 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    let t: [i16; 3] = [1, 2, a];
    for (i, x) in t.iter().enumerate() {
        out(i as i16 * 100 + *x);
    }
}
