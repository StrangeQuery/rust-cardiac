//! feature: A `const` as a pattern
//! section: Path patterns
//! input: 7 3 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
const LUCKY: i16 = 7;
pub fn main() {
    loop {
        let v = inp();
        if v == 0 { break; }
        out(match v { LUCKY => 1, _ => 0 });
    }
}
