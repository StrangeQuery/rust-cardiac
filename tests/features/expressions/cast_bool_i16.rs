//! feature: `bool as i16`
//! section: Type cast expressions
//! input: 3 -3 -1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    loop {
        let a = inp();
        out((a > 0) as i16 + 10);
        if a == -1 {
            break;
        }
    }
}
