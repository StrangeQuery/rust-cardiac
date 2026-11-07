//! feature: Array pattern (`let [a, b, c] = t;`)
//! section: Slice patterns
//! input: 1 2 3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let t = [inp(), inp(), inp()];
    let [a, b, c] = t;
    out(c * 100 + b * 10 + a);
}
