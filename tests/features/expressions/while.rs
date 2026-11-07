//! feature: `while`
//! section: Loop expressions
//! input: 3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let mut n = inp();
    while n >= 0 {
        out(n);
        n -= 1;
    }
}
