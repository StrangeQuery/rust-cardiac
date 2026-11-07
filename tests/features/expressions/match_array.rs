//! feature: `match` on an array pattern, `[a, 0, _]`
//! section: Match expressions
//! input: 1 0 3 1 2 3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    for _ in 0..2 {
        let t = [inp(), inp(), inp()];
        out(match t { [a, 0, _] => a, [_, b, c] => b + c });
    }
}
