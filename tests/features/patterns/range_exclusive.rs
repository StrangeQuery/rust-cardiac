//! feature: Exclusive `a..b` in a `match`
//! section: Range patterns
//! input: 0 9 10 -1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    for _ in 0..4 {
        let v = inp();
        let r = match v {
            0..10 => 1,
            _ => 0,
        };
        out(r);
    }
}
