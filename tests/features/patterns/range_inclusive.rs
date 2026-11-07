//! feature: `a..=b` in a `match`
//! section: Range patterns
//! input: -5 0 3 50 999 -999 1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    for _ in 0..7 {
        let v = inp();
        let r = match v {
            -999..=-1 => 1,
            0..=9 => 2,
            10..=99 => 3,
            _ => 4,
        };
        out(r);
    }
}
