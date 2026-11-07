//! feature: `match` with range patterns, `a..=b` and `..0`
//! section: Match expressions
//! input: -5 0 7 50 999
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    for _ in 0..5 {
        let a = inp();
        let r = match a { ..0 => -1, 0 => 0, 1..=9 => 1, 10..=99 => 2, _ => 3 };
        out(r);
    }
}
