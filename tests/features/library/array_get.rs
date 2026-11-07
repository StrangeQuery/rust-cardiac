//! feature: `array.get(i)`, which returns an `Option`
//! section: Arrays and slices
//! disposition: demand
//! note: It returns an `Option<&i16>`, and an enum's fields must be scalars for now.
//! input: 3 -1 4 1 2 9
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let t: [i16; 4] = [inp(), inp(), inp(), inp()];
    for _ in 0..2 {
        let i = inp() as usize;
        out(match t.get(i) { Some(&v) => v, None => -50 });
    }
}
