//! feature: `_` inside a tuple (`(_, b)`)
//! section: Wildcard pattern
//! input: 4 7 -1 2
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    for _ in 0..2 {
        let t = (inp(), inp());
        let (_, b) = t;
        out(b);
    }
}
