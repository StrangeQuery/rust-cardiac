//! feature: A struct with named fields, passed by value
//! section: Structs
//! input: 3 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[derive(Clone, Copy)]
struct P { x: i16, y: i16 }
#[inline(never)]
fn manhattan(p: P) -> i16 { p.x.abs() + p.y.abs() }
pub fn main() {
    let p = P { x: inp(), y: -inp() };
    out(manhattan(p));
}
