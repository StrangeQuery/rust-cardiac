//! feature: `let x;` assigned in each arm of a `match`
//! section: Let statements
//! input: 1 2 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    for _ in 0..3 {
        let a = inp();
        let x: i16;
        match a {
            1 => x = 10,
            2 => x = 20,
            _ => x = a,
        }
        out(x);
    }
}
