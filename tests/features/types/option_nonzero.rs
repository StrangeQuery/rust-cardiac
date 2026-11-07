//! feature: `Option<NonZeroI16>`, whose `None` is zero
//! section: Type layout
//! disposition: demand
//! input: 5 0
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
use core::num::NonZeroI16;
pub fn main() {
    let mut n = 0;
    while n < 2 {
        match NonZeroI16::new(inp()) {
            Some(v) => out(v.get()),
            None => out(-1),
        }
        n += 1;
    }
}
