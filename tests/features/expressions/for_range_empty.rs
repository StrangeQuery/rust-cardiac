//! feature: `for i in a..b` with `a >= b`: no passes
//! section: Loop expressions
//! input: 5 5 6 2
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    for _ in 0..2 {
        let a = inp();
        let b = inp();
        for i in a..b {
            out(i);
        }
        out(-1);
    }
}
