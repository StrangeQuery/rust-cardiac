//! feature: A byte string literal (`b"CARD"`), indexed by an input
//! section: Integer literals
//! disposition: easy
//! note: A byte string literal is a constant `u8` table, which the compiler already has for `const` arrays.
//! input: 0 3 1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let s: &[u8; 4] = b"CARD";
    let mut k = 0;
    while k < 3 {
        let i = inp() as usize;
        let c = s[i];
        out(c as i16);
        k += 1;
    }
}
