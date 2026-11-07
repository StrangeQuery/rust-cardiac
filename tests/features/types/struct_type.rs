//! feature: A struct type with named fields
//! section: Struct types
//! input: 3 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
struct P {
    x: i16,
    y: i16,
}
pub fn main() {
    let p = P { x: inp(), y: inp() };
    out(p.x - p.y);
}
