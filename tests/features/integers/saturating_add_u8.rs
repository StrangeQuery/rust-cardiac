//! feature: `a.saturating_add(b)` on `u8`
//! section: Saturating
//! disposition: easy
//! note: 255 fits on a card, so this is an add and a compare; rustc uses the `saturating_add` intrinsic, which the compiler doesn't know.
//! input: 1 2 5 5 6 6 7 1 -1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
const U: [u8; 8] = [0, 1, 2, 7, 100, 200, 254, 255];
pub fn main() {
    loop {
        let i = inp();
        if i < 0 {
            break;
        }
        let a = U[i as usize];
        let b = U[inp() as usize];
        out(a.saturating_add(b) as i16);
    }
}
