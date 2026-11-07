//! feature: `let x;` without an initialiser, assigned later
//! section: Let statements
//! input: 7 3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let x: i16;
    let a = inp();
    x = a + inp();
    out(x);
}
