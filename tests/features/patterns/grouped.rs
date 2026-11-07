//! feature: Grouped pattern (`&(0..=5)`)
//! section: Grouped patterns
//! input: 3 7
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    for _ in 0..2 {
        let v = inp();
        let r = &v;
        let k = match r {
            &(0..=5) => 1,
            _ => 0,
        };
        out(k);
    }
}
