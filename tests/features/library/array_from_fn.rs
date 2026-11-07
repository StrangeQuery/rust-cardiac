//! feature: `core::array::from_fn`
//! section: Arrays and slices
//! disposition: demand
//! input: 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    let t: [i16; 4] = core::array::from_fn(|i| i as i16 + a);
    for i in 0..t.len() {
        out(t[i]);
    }
}
