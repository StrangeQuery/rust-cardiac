//! feature: `use m::{self, f}` (nested groups with `self`)
//! section: Use declarations
//! input: 10
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
mod outer {
    pub mod m {
        pub fn half(x: i16) -> i16 { x / 2 }
        pub fn dec(x: i16) -> i16 { x - 1 }
    }
}
use outer::m::{self, half};
pub fn main() {
    let v = inp();
    out(half(v));
    out(m::dec(v));
}
