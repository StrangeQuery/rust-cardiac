//! feature: `array.contains(&x)`
//! section: Arrays and slices
//! disposition: easy
//! note: On an array of known length it is a short loop the compiler already writes for `for i in 0..N`.
//! input: 3 -1 4 1 4 7
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let t: [i16; 4] = [inp(), inp(), inp(), inp()];
    for _ in 0..2 {
        let x = inp();
        out(t.contains(&x) as i16);
    }
}
