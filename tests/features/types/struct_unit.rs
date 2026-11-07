//! feature: A unit struct passed to a function
//! section: Struct types
//! disposition: easy
//! note: A unit struct is no cells at all.
//! input: 7
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
struct Marker;
#[inline(never)]
fn show(_m: Marker, x: i16) {
    out(x + 1);
}
pub fn main() {
    show(Marker, inp());
}
