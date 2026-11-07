//! feature: `#[inline(always)]` on a helper
//! section: Functions
//! input: 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(always)]
fn add3(x: i16) -> i16 { x + 3 }
pub fn main() {
    let v = inp();
    out(add3(v));
    out(add3(add3(v)));
}
