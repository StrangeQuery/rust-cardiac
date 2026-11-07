//! feature: `assert_ne!(a, b)`: passing, then failing
//! section: Assertions
//! disposition: easy
//! note: rustc calls `core::panicking::assert_failed` with both values; the compiler could treat it as the panic of `assert!(a == b)`.
//! input: 7 3 7 7
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    for _ in 0..2 {
        let a = inp();
        let b = inp();
        assert_ne!(a, b);
        out(a);
    }
}
