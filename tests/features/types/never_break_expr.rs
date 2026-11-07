//! feature: `break` as an expression of type `!` in one arm of an `if`
//! section: Never type
//! input: 4 -2 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    loop {
        let a = inp();
        let b: i16 = if a == 0 { break } else { a * 2 };
        out(b);
    }
}
