//! feature: `#[allow(...)]`, `#[warn(...)]`, `#[deny(...)]`, `#[forbid(...)]` on items
//! section: Diagnostic attributes
//! input: 5
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[allow(dead_code)]
fn unused() -> i16 { 1 }
#[deny(unused_must_use)]
#[warn(unused_variables)]
#[forbid(unsafe_code)]
pub fn main() {
    #[allow(unused_mut)]
    let mut a = inp();
    out(a);
}
