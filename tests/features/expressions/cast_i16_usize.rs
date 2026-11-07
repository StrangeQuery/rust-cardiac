//! feature: `i16 as usize` and `usize as i16`
//! section: Type cast expressions
//! input: 0 5 998 -1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    loop {
        let a = inp();
        if a < 0 {
            break;
        }
        let u = a as usize;
        out((u + 1) as i16);
    }
}
