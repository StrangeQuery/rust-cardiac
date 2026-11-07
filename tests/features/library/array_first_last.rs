//! feature: `array.first()` and `last()`
//! section: Arrays and slices
//! disposition: demand
//! note: They return an `Option<&i16>`, and an enum's fields must be scalars for now.
//! input: 3 -1 4 1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let t: [i16; 4] = [inp(), inp(), inp(), inp()];
    out(*t.first().unwrap());
    out(*t.last().unwrap());
}
