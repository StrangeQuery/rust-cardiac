//! feature: shadowing: `let a = a + 1;`
//! section: Let statements
//! input: 7
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    out(a);
    let a = a + 1;
    out(a);
    let a = a * 10;
    out(a);
}
