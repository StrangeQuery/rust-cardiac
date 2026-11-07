//! feature: `match` on an `i16` with literal arms and `_`
//! section: Match expressions
//! input: 1 2 3 -7
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    for _ in 0..4 {
        let a = inp();
        let r = match a { 1 => 10, 2 => 20, 3 => 30, _ => 0 };
        out(r);
    }
}
