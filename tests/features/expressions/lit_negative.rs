//! feature: A negative literal (`-999`) and the largest a card holds (`999`)
//! section: Literal expressions
//! input: 0 1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    let b = inp();
    out(if a == 0 { -999 } else { 0 });
    out(if b == 0 { 0 } else { 999 });
}
