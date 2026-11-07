//! feature: `a.overflowing_add(b)` on `u8`
//! section: Overflowing
//! disposition: easy
//! note: A wrong answer at opt-level 2: the flag is right but the sum isn't wrapped, so 200 + 100 prints 300 where Rust prints 44. At opt-level 1 it is a call.
//! input: 1 2 5 4 7 7 -1
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
        let (v, o) = a.overflowing_add(b);
        out(v as i16);
        out(o as i16);
    }
}
