//! feature: A byte literal (`b'A'`)
//! section: Literal expressions
//! input: 1 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    loop {
        let a = inp();
        if a == 0 { break; }
        let c: u8 = if a > 0 { b'A' } else { b'z' };
        out(c as i16);
    }
}
