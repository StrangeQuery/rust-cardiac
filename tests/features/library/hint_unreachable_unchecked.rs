//! feature: `core::hint::unreachable_unchecked`
//! section: Hints (core::hint)
//! input: 0 1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    for _ in 0..2 {
        let a = inp();
        out(match a {
            0 => 5,
            1 => 9,
            _ => unsafe { core::hint::unreachable_unchecked() },
        });
    }
}
