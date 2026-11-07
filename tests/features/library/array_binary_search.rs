//! feature: `array.binary_search(&x)`
//! section: Arrays and slices
//! disposition: demand
//! input: 2 5 9 4 9 6
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let t: [i16; 4] = [inp(), inp(), inp(), inp()];
    for _ in 0..2 {
        let x = inp();
        out(match t.binary_search(&x) { Ok(i) => i as i16, Err(i) => -(i as i16) - 1 });
    }
}
