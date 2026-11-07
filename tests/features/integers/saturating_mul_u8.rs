//! feature: `a.saturating_mul(b)` on `u8`
//! section: Saturating
//! disposition: demand
//! note: At opt-level 2 rustc's inlined version doesn't fit in the cells; a multiply and a compare would.
//! input: 3 2
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
const U: [u8; 8] = [0, 1, 2, 7, 100, 200, 254, 255];
pub fn main() {
    let a = U[inp() as usize];
    let b = U[inp() as usize];
    out(a.saturating_mul(b) as i16);
}
