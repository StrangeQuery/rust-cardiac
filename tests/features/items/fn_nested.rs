//! feature: A function defined inside another
//! section: Functions
//! input: 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    #[inline(never)]
    fn helper(x: i16) -> i16 { x * 10 }
    let v = inp();
    out(helper(v));
    out(helper(v + 1));
}
