//! feature: `true` and `false` patterns
//! section: Literal patterns
//! input: 3 -3 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    loop {
        let v = inp();
        if v == 0 { break; }
        let r = match v > 0 {
            true => 1,
            false => -1,
        };
        out(r);
    }
}
