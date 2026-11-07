//! feature: A `pub struct` with `pub` fields from a module
//! section: Modules
//! input: 3 9
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
mod geo {
    pub struct P { pub x: i16, pub y: i16 }
    pub fn sum(p: &P) -> i16 { p.x + p.y }
}
pub fn main() {
    let p = geo::P { x: inp(), y: inp() };
    out(geo::sum(&p));
    out(p.x);
}
