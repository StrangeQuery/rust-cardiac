//! feature: `a.midpoint(b)` on `i16`
//! section: Powers and roots
//! disposition: never
//! note: Refused on purpose: rustc adds the two in `i32` and halves, and where the sum passes a card (999 and 998) CARDIAC would print 598 where Rust prints 998. `(a + b) / 2` works where the sum stays on a card.
//! input: 7 3 -7 3 999 998 -5 -6 0 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    loop {
        let a = inp();
        let b = inp();
        if a == 0 && b == 0 {
            break;
        }
        out(a.midpoint(b));
    }
}
