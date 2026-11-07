//! feature: `t.len()`
//! section: Array and array index expressions
//! input: 2
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let t = [1i16, 2, 3, 4, 5];
    let a = inp();
    out(t.len() as i16 + a);
}
