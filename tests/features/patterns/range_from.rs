//! feature: Half-open `a..` in a `match`
//! section: Range patterns
//! input: -3 0 400
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    for _ in 0..3 {
        let v = inp();
        let r = match v {
            100.. => 2,
            0.. => 1,
            _ => 0,
        };
        out(r);
    }
}
