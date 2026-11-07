//! feature: `for x in array.iter_mut()`
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
    let mut t: [i16; 3] = [1, 2, a];
    for x in t.iter_mut() {
        *x += 10;
    }
    out(t[0]);
    out(t[1]);
    out(t[2]);
}
