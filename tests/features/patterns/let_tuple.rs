//! feature: `let (a, b) = (x, y);`
//! section: Destructuring
//! input: 7 -3 0 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    for _ in 0..2 {
        let (a, b) = (inp(), inp());
        out(a - b);
        out(b);
    }
}
