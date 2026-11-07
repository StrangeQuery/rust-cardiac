//! feature: A labelled block, `'a: { ... break 'a v; ... }`
//! section: Loop expressions
//! input: 3 -3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    for _ in 0..2 {
        let a = inp();
        let r = 'done: {
            if a < 0 {
                break 'done -1;
            }
            a * 2
        };
        out(r);
    }
}
