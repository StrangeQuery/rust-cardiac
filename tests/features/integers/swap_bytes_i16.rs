//! feature: `a.swap_bytes()` on `i16`
//! section: Bits
//! disposition: never
//! note: Swapping an i16's bytes moves its low byte to the high one: most values on a card come out far past one (100 becomes 25600), so the result can't be held.
//! input: 256 512 768 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    loop {
        let a = inp();
        out(a.swap_bytes());
        if a == 0 {
            break;
        }
    }
}
