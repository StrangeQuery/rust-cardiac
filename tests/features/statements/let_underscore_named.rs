//! feature: `let _x = e;` (an unused binding)
//! section: Let statements
//! input: 7 3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let _x = inp();
    let b = inp();
    out(b);
}
