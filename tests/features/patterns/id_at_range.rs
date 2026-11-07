//! feature: `x @ 1..=5`
//! section: Identifier patterns
//! input: 3 9 -2 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    loop {
        let v = inp();
        if v == 0 { break; }
        let r = match v {
            x @ 1..=5 => x * 10,
            x => -x,
        };
        out(r);
    }
}
