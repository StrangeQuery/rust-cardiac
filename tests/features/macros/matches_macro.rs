//! feature: `matches!(x, pattern)`
//! section: Other core macros
//! input: 3 12 -1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    for _ in 0..3 {
        let a = inp();
        out(matches!(a, 1..=9) as i16);
    }
}
