//! feature: `let x;` assigned in each branch of an `if`
//! section: Let statements
//! input: 7 -3 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    for _ in 0..3 {
        let a = inp();
        let x;
        if a < 0 {
            x = -1;
        } else if a == 0 {
            x = 0;
        } else {
            x = a * 2;
        }
        out(x);
    }
}
