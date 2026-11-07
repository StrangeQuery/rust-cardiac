//! feature: `assert_eq!(a, b, "message")`
//! section: Assertions
//! disposition: easy
//! note: The message is a literal, which could go in the panic map as `unwrap()`'s and `expect()`'s messages do.
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
        assert_eq!(a, b, "the cards differ");
        out(a);
    }
}
