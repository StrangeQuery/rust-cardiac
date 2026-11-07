//! feature: `debug_assert_eq!` and `debug_assert_ne!`
//! section: Assertions
//! debug-assertions: on
//! input: 7 7 7 3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    for _ in 0..2 {
        let a = inp();
        let b = inp();
        debug_assert_ne!(a, 0);
        debug_assert_eq!(a, b);
        out(a);
    }
}
