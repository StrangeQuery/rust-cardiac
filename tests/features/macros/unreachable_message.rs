//! feature: `unreachable!("message")`
//! section: Panics
//! disposition: easy
//! note: The message is a literal, which could go in the panic map as `unwrap()`'s and `expect()`'s messages do.
//! input: 1 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    for _ in 0..2 {
        let a = inp();
        let v = match a {
            1 => 10,
            _ => unreachable!("only ones here"),
        };
        out(v);
    }
}
