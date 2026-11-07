//! feature: `PartialOrd` implemented by hand
//! section: Traits
//! disposition: demand
//! note: `partial_cmp` returns `Option<Ordering>`, and rustc turns `<` into a three-way compare the compiler doesn't handle yet.
//! input: 5 -7
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
use core::cmp::Ordering;
#[derive(PartialEq)]
struct Mag(i16);
impl PartialOrd for Mag {
    fn partial_cmp(&self, o: &Mag) -> Option<Ordering> { self.0.abs().partial_cmp(&o.0.abs()) }
}
pub fn main() {
    let a = Mag(inp());
    let b = Mag(inp());
    out((a < b) as i16);
    out((a > b) as i16);
}
