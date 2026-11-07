//! feature: A `static` scalar
//! section: Static items
//! input: 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
static BASE: i16 = 300;
pub fn main() {
    out(BASE + inp());
}
