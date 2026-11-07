//! feature: `if` and `match` as statements without a semicolon
//! section: Expression statements
//! input: 7 -3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    for _ in 0..2 {
        let a = inp();
        if a > 0 {
            out(1)
        } else {
            out(-1)
        }
        match a {
            7 => out(70),
            _ => out(0),
        }
    }
}
