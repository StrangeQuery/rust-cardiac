//! feature: A repeat expression `[v; N]`
//! section: Array and array index expressions
//! input: 7 3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let v = inp();
    let mut t = [v; 5];
    let i = inp() as usize;
    t[i] = 0;
    out(t[0] + t[1] + t[2] + t[3] + t[4]);
}
