//! feature: `for i in a..b` with negative bounds
//! section: Loop expressions
//! input: -3 2
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    let b = inp();
    for i in a..b {
        out(i);
    }
}
