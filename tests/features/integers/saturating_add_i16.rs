//! feature: `a.saturating_add(b)` on `i16`, on a card
//! section: Saturating
//! disposition: demand
//! note: Rust saturates at 32767, which no card holds, so past a card CARDIAC would print something else; on a card it is a plain add.
//! input: 7 3 -7 3 500 499 0 0
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
        out(a.saturating_add(b));
    }
}
