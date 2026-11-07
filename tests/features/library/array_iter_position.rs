//! feature: `array.iter().position(...)`
//! section: Arrays and slices
//! disposition: easy
//! note: The same loop as `for i in 0..N` with an early return of `i`.
//! input: 3 -1 4 1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let t: [i16; 4] = [inp(), inp(), inp(), inp()];
    out(match t.iter().position(|&x| x < 0) { Some(i) => i as i16, None => -1 });
    out(match t.iter().position(|&x| x > 50) { Some(i) => i as i16, None => -1 });
}
