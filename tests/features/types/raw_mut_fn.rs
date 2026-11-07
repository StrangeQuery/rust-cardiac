//! feature: A raw pointer `*mut i16` written through in a function
//! section: Pointer types
//! input: 9
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn set(p: *mut i16, v: i16) {
    unsafe { *p = v; }
}
pub fn main() {
    let mut a = 0;
    set(&mut a, inp());
    out(a * 2);
}
