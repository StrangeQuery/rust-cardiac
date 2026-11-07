//! feature: `match` arms with guards, `x if x > b`
//! section: Match expressions
//! input: 5 3 3 5 4 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    for _ in 0..3 {
        let a = inp();
        let b = inp();
        out(match a { x if x > b => 1, x if x < b => -1, _ => 0 });
    }
}
