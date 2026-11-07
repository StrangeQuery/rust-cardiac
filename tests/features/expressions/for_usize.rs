//! feature: `for i in 0..n` over `usize`, indexing an array
//! section: Loop expressions
//! input: 3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let t: [i16; 5] = [10, 20, 30, 40, 50];
    let n = inp() as usize;
    for i in 0..n {
        out(t[i]);
    }
}
