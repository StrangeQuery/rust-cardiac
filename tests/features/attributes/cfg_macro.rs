//! feature: `cfg!(...)` as a value
//! section: Conditional compilation
//! input: 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    if cfg!(all()) { out(a); }
    if cfg!(any()) { out(-a); }
}
