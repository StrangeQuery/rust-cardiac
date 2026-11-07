//! feature: An `if` / `else if` / `else` chain
//! section: If and if let expressions
//! input: -5 0 5 50 500 999
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    for _ in 0..6 {
        let a = inp();
        let c = if a < 0 { -1 } else if a == 0 { 0 } else if a < 10 { 1 } else if a < 100 { 2 } else { 3 };
        out(c);
    }
}
