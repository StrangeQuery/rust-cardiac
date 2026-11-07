//! feature: `t[i]` read and written with `i` from a card
//! section: Array and array index expressions
//! input: 3 9 0 8 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let mut t = [0i16; 6];
    loop {
        let i = inp() as usize;
        if i == 0 { break; }
        t[i] = inp();
    }
    out(t[3]);
    out(t[0]);
}
