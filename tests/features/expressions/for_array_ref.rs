//! feature: `for x in &array` and `array.iter()`
//! section: Loop expressions
//! disposition: easy
//! note: The same as `for i in 0..N` with `t[i]`.
//! input: 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    let t: [i16; 4] = [1, 2, 3, a];
    for x in &t {
        out(*x);
    }
    for x in t.iter() {
        out(*x + 1);
    }
}
