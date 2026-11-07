//! feature: `loop` with `break value`
//! section: Loop expressions
//! input: 4 9 2 -1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let first_big = loop {
        let a = inp();
        if a > 5 || a < 0 {
            break a;
        }
    };
    out(first_big);
}
