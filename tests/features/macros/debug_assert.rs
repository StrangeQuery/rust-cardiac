//! feature: `debug_assert!(cond)`
//! section: Assertions
//! debug-assertions: on
//! input: 7 -3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    for _ in 0..2 {
        let a = inp();
        debug_assert!(a > 0);
        out(a);
    }
}
