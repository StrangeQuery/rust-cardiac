//! feature: `match` binding with `@`, `n @ 1..=9`
//! section: Match expressions
//! input: 7 70
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    for _ in 0..2 {
        let a = inp();
        out(match a { n @ 1..=9 => n * 2, n => n + 1 });
    }
}
