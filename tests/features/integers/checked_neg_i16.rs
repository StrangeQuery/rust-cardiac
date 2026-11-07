//! feature: `a.checked_neg()` on `i16`
//! section: Checked
//! disposition: easy
//! note: The compiler crashes at opt-level 2: rustc's inlined test against `i16::MIN` puts -32768 into a card, and the compiler panics instead of refusing it. No card holds `i16::MIN`, so on CARDIAC the test is never true. At opt-level 1 it is a call.
//! input: 7 -7 999 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    loop {
        let a = inp();
        match a.checked_neg() {
            Some(v) => out(v as i16),
            None => out(-1),
        }
        if a == 0 {
            break;
        }
    }
}
