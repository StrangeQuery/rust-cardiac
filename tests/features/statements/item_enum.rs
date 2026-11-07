//! feature: an `enum` declared inside a function
//! section: Item declarations
//! input: 7 -3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    enum Sign {
        Plus,
        Minus,
    }
    for _ in 0..2 {
        let a = inp();
        let s = if a < 0 { Sign::Minus } else { Sign::Plus };
        match s {
            Sign::Plus => out(1),
            Sign::Minus => out(-1),
        }
    }
}
