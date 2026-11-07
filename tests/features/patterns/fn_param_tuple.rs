//! feature: `fn f((a, b): (i16, i16))`
//! section: Destructuring
//! input: 10 3 -4 6
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn diff((a, b): (i16, i16)) -> i16 { a - b }
pub fn main() {
    for _ in 0..2 {
        let a = inp();
        let b = inp();
        out(diff((a, b)));
    }
}
