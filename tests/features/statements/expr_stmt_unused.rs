//! feature: an unused arithmetic result (`a + b;`)
//! section: Expression statements
//! input: 7 3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[allow(unused_must_use)]
pub fn main() {
    let a = inp();
    let b = inp();
    #[allow(unused)]
    {
        a + b;
    }
    out(a - b);
}
