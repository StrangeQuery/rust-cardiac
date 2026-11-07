//! feature: `mut x` binding
//! section: Identifier patterns
//! input: 5 -2 0
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
            mut x => { x += 100; x }
        };
        out(r);
    }
}
