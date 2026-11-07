//! feature: `loop` with `break`
//! section: Loop expressions
//! input: 3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let mut n = inp();
    loop {
        out(n);
        n -= 1;
        if n < 0 {
            break;
        }
    }
}
