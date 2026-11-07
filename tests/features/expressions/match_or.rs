//! feature: `match` with `|` alternatives
//! section: Match expressions
//! input: 1 2 3 4 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    for _ in 0..5 {
        let a = inp();
        out(match a { 1 | 3 | 5 => 1, 2 | 4 => 2, _ => 0 });
    }
}
