//! feature: A struct literal with named fields
//! section: Struct expressions
//! input: 3 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
struct P { x: i16, y: i16 }
#[inline(never)]
fn sum(p: P) -> i16 { p.x + p.y }
pub fn main() {
    let p = P { x: inp(), y: inp() };
    out(sum(p));
}
