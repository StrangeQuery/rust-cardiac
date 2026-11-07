//! feature: An `unsafe` block as a value
//! section: Block expressions
//! input: 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let mut v: i16 = inp();
    let p = &mut v as *mut i16;
    let w = unsafe { *p += 1; *p };
    out(w);
}
