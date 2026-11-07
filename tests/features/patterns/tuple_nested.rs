//! feature: Nested tuple pattern (`((a, b), c)`)
//! section: Tuple patterns
//! input: 1 2 3
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
pub fn main() {
    let t = ((inp(), inp()), inp());
    let ((a, b), c) = t;
    out(a * 100 + b * 10 + c);
}
