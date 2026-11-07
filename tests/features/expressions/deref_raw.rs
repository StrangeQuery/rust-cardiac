//! feature: `*p` through a raw pointer to an array element
//! section: Dereference operator
//! input: 2
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let mut t = [1, 2, 3, 4];
    let i = inp() as usize;
    let p = &mut t[i] as *mut i16;
    unsafe { *p = 77; }
    out(t[0] + t[1] + t[2] + t[3]);
}
