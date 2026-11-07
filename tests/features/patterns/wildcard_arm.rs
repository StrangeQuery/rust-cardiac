//! feature: `_` as the last arm
//! section: Wildcard pattern
//! input: 1 5 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    loop {
        let v = inp();
        if v == 0 { break; }
        out(match v { 1 => 1, _ => 2 });
    }
}
