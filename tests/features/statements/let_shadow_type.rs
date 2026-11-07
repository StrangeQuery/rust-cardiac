//! feature: shadowing with a new type (`i16` to `bool` to `u8`)
//! section: Let statements
//! input: 7 -3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    for _ in 0..2 {
        let a = inp();
        let a = a > 0;
        let a = a as u8;
        out(a as i16);
    }
}
