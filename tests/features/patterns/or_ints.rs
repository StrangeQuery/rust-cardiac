//! feature: `1 | 2 =>`
//! section: Or-patterns
//! input: 1 2 3 4 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    loop {
        let v = inp();
        if v == 0 { break; }
        out(match v { 1 | 2 => 1, 3 | 4 => 2, _ => 0 });
    }
}
