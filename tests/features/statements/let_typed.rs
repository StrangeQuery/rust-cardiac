//! feature: `let x: i16 = e;` with a type
//! section: Let statements
//! input: 7 -3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a: i16 = inp();
    let b: i16 = inp();
    out(a - b);
}
