//! feature: `for i in (a..b).step_by(k)`
//! section: Loop expressions
//! disposition: easy
//! note: A `while` loop that adds `k` does the same.
//! input: 0 10 3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    let b = inp();
    let k = inp() as usize;
    for i in (a..b).step_by(k) {
        out(i);
    }
}
