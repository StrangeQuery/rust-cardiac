//! feature: `panic!()`
//! section: Panics
//! input: 7 -3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    for _ in 0..2 {
        let a = inp();
        if a < 0 {
            panic!();
        }
        out(a);
    }
}
