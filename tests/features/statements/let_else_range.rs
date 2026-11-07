//! feature: let-else with a refutable range pattern
//! section: Let-else
//! input: 5 12 3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    for _ in 0..3 {
        let a = inp();
        let v @ 1..=9 = a else {
            out(-1);
            continue;
        };
        out(v);
    }
}
