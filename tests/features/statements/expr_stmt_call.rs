//! feature: a call as a statement, its result unused
//! section: Expression statements
//! input: 7 3 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    inp();
    let b = inp();
    inp();
    out(b);
}
