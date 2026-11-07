//! feature: A 2-D grid in one flat array, `t[r * W + c]`
//! section: Array and array index expressions
//! input: 1 2
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
const W: usize = 3;
pub fn main() {
    let mut t = [0i16; 9];
    for r in 0..3 {
        for c in 0..W {
            t[r * W + c] = (r * 10 + c) as i16;
        }
    }
    let r = inp() as usize;
    let c = inp() as usize;
    out(t[r * W + c]);
}
