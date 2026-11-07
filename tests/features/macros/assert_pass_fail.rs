//! feature: `assert!(cond)`: passing, then failing
//! section: Assertions
//! input: 7 -3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    for _ in 0..2 {
        let a = inp();
        assert!(a > 0);
        out(a);
    }
}
