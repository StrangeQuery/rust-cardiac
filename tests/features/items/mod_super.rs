//! feature: `super::` and `self::` paths
//! section: Modules
//! input: 6
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
fn base(x: i16) -> i16 { x * 2 }
mod m {
    fn local(x: i16) -> i16 { x + 1 }
    pub fn f(x: i16) -> i16 { self::local(super::base(x)) }
}
pub fn main() {
    out(m::f(inp()));
}
