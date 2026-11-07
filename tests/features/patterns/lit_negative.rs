//! feature: Negative integer literals (`-5 =>`)
//! section: Literal patterns
//! input: -5 -1 5 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    loop {
        let v = inp();
        let r = match v {
            -5 => 1,
            -1 => 2,
            0 => break,
            _ => 3,
        };
        out(r);
    }
}
