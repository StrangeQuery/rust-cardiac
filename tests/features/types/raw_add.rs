//! feature: Pointer arithmetic: `p.add(i)` into an array
//! section: Pointer types
//! disposition: demand
//! input: 4 5 6 2
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let t = [inp(), inp(), inp()];
    let i = inp() as usize;
    let p = (&t as *const [i16; 3]).cast::<i16>();
    out(unsafe { *p.add(i) });
}
