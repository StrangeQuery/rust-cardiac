//! feature: `slice.split_at(k)`
//! section: Arrays and slices
//! disposition: demand
//! note: It returns two slices, which the compiler doesn't have.
//! input: 3 -1 4 1 1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let t: [i16; 4] = [inp(), inp(), inp(), inp()];
    let (l, r) = t.split_at(inp() as usize);
    out(l.len() as i16);
    out(r[0]);
}
