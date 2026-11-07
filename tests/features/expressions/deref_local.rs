//! feature: `*r` through a local reference, reading and writing
//! section: Dereference operator
//! input: 8
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let mut a = inp();
    {
        let r = &mut a;
        *r = *r * 2 + 1;
    }
    let s = &a;
    out(*s);
}
