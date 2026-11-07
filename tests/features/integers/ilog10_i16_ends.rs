//! feature: `a.ilog10()` on `i16` at every boundary, then a negative: Rust's panic
//! section: Powers and roots
//! input: 1 2 9 10 11 99 100 101 998 999 -5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    loop {
        let a = inp();
        out(a.ilog10() as i16);
    }
}
