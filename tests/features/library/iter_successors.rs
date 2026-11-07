//! feature: `core::iter::successors`
//! section: Iterators
//! disposition: demand
//! input: 3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    let it = core::iter::successors(Some(a), |&x| if x < 100 { Some(x * 2) } else { None });
    out(it.count() as i16);
}
