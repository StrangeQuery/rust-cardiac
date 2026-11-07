//! feature: `use m::*` (glob import)
//! section: Use declarations
//! input: 4 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
mod m {
    pub fn add(a: i16, b: i16) -> i16 { a + b }
    pub fn sub(a: i16, b: i16) -> i16 { a - b }
    pub const K: i16 = 100;
}
use m::*;
pub fn main() {
    let a = inp();
    let b = inp();
    out(add(a, b));
    out(sub(a, b));
    out(add(a, K));
}
