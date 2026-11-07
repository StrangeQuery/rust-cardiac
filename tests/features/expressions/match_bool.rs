//! feature: `match` on a `bool`
//! section: Match expressions
//! input: 1 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    for _ in 0..2 {
        let b = inp() != 0;
        out(match b { true => 7, false => 9 });
    }
}
