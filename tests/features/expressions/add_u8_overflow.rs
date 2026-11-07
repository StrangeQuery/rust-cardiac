//! feature: `a + b` on `u8` past 255: panics, or wraps with checks off
//! section: Arithmetic operators
//! input: 5 5 -1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
const U: [u8; 8] = [0, 1, 2, 7, 100, 200, 254, 255];
pub fn main() {
    loop {
        let i = inp();
        if i < 0 {
            break;
        }
        let a = U[i as usize];
        let b = U[inp() as usize];
        let c = a + b;
        out(c as i16);
    }
}
