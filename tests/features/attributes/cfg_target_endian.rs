//! feature: `cfg!(target_endian = "little")`
//! section: Conditional compilation
//! input: 1
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let a = inp();
    if cfg!(target_endian = "little") { out(a); }
    if cfg!(target_endian = "big") { out(-a); }
}
