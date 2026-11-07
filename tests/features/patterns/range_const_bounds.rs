//! feature: Consts as range bounds (`LO..=HI`)
//! section: Range patterns
//! input: 5 25 -5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
const LO: i16 = 1;
const HI: i16 = 20;
pub fn main() {
    for _ in 0..3 {
        let v = inp();
        let r = match v {
            LO..=HI => 1,
            _ => 0,
        };
        out(r);
    }
}
