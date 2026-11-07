//! feature: `t.iter().zip(u.iter())`
//! section: Iterators
//! disposition: demand
//! input: 1 2 3 10 20 30
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let t: [i16; 3] = [inp(), inp(), inp()];
    let u: [i16; 3] = [inp(), inp(), inp()];
    out(t.iter().zip(u.iter()).map(|(x, y)| x + y).sum::<i16>());
}
