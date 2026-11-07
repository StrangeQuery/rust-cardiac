//! feature: An index past the end panics
//! section: Array and array index expressions
//! input: 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let t = [1, 2, 3];
    let i = inp() as usize;
    out(t[i]);
}
