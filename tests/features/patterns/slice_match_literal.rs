//! feature: Array pattern with literals in a `match` (`[0, _, x]`)
//! section: Slice patterns
//! input: 0 5 6 1 0 7 2 2 2
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    for _ in 0..3 {
        let t = [inp(), inp(), inp()];
        let r = match t {
            [0, _, x] => x,
            [_, 0, x] => -x,
            _ => 0,
        };
        out(r);
    }
}
