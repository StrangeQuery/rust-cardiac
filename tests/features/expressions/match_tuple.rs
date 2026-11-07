//! feature: `match` on a tuple of two values
//! section: Match expressions
//! input: 0 0 0 5 5 0 5 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    for _ in 0..4 {
        let a = inp();
        let b = inp();
        out(match (a, b) { (0, 0) => 0, (0, _) => 1, (_, 0) => 2, _ => 3 });
    }
}
