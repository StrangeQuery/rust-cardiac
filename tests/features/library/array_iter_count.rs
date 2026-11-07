//! feature: `array.iter().filter(...).count()`
//! section: Arrays and slices
//! disposition: demand
//! note: A `filter` adaptor over a slice iterator.
//! input: 3 -1 4 1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let t: [i16; 4] = [inp(), inp(), inp(), inp()];
    out(t.iter().filter(|&&x| x > 1).count() as i16);
}
