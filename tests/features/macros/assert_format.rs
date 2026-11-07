//! feature: `assert!(cond, "{}", x)` with a formatted message
//! section: Assertions
//! disposition: demand
//! note: The check could keep the message's literal text and leave out the values; formatting the values themselves is never.
//! input: 7 -3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    for _ in 0..2 {
        let a = inp();
        assert!(a > 0, "a was {}", a);
        out(a);
    }
}
