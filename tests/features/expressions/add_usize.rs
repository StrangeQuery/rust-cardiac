//! feature: `a + b` on `usize`
//! section: Arithmetic operators
//! input: 7 3 100 9 3 3 0 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    loop {
        let a = inp() as usize;
        let b = inp() as usize;
        if b == 0 {
            break;
        }
        out((a + b) as i16);
    }
}
