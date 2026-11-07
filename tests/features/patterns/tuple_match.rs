//! feature: Tuple pattern in a `match` (`(0, y)`)
//! section: Tuple patterns
//! input: 0 5 5 0 2 3 9 9
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    for _ in 0..4 {
        let a = inp();
        let b = inp();
        let r = match (a, b) {
            (0, y) => y,
            (x, 0) => -x,
            (x, y) if x == y => 0,
            (x, y) => x * 10 + y,
        };
        out(r);
    }
}
