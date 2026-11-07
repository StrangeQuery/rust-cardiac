//! feature: A tuple passed as a parameter
//! section: Functions
//! input: 3 8
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[inline(never)]
fn span(p: (i16, i16)) -> i16 { p.1 - p.0 }
pub fn main() {
    let p = (inp(), inp());
    out(span(p));
}
