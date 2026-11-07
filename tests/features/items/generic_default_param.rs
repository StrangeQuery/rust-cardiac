//! feature: A generic type parameter with a default (`struct W<T = i16>`)
//! section: Generic parameters
//! input: 6
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
struct W<T = i16> { v: T }
fn get(w: W) -> i16 { w.v }
pub fn main() {
    out(get(W { v: inp() }) + 1);
}
