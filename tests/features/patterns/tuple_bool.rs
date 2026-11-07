//! feature: Tuple of `bool`s (`(true, false) =>`)
//! section: Tuple patterns
//! input: 1 1 1 -1 -1 1 -1 -1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    for _ in 0..4 {
        let a = inp() > 0;
        let b = inp() > 0;
        let r = match (a, b) {
            (true, true) => 3,
            (true, false) => 2,
            (false, true) => 1,
            (false, false) => 0,
        };
        out(r);
    }
}
