//! feature: A raw pointer `*const i16` read with `unsafe`
//! section: Pointer types
//! input: 9
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    let p: *const i16 = &a;
    out(unsafe { *p } + 1);
}
