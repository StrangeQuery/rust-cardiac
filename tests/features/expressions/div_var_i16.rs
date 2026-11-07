//! feature: `a / b` on `i16`, both variables
//! section: Arithmetic operators
//! note: A divide routine, put in once, runs as many steps as the quotient. Rounds toward zero, as Rust does.
//! input: 7 3 -7 3 7 -3 -7 -3 999 -999 0 5 9 10 500 499 -1 1 0 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    loop {
        let a = inp();
        let b = inp();
        if a == 0 && b == 0 {
            break;
        }
        out(a / b);
    }
}
