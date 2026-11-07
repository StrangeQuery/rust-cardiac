//! feature: `a * 7`: by a constant
//! section: Arithmetic operators
//! input: 7 -7 99 -99 142 -142 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    loop {
        let a = inp();
        out(a * 7);
        if a == 0 {
            break;
        }
    }
}
