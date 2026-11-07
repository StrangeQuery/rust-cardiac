//! feature: An enum with a struct-like variant (`V { x, y }`)
//! section: Enumerations
//! disposition: easy
//! note: A variant with named fields is laid out as one with unnamed fields, which already works.
//! input: 1 3 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
enum E { None, At { x: i16, y: i16 } }
#[inline(never)]
fn read() -> E { if inp() == 1 { E::At { x: inp(), y: inp() } } else { E::None } }
pub fn main() {
    match read() {
        E::At { x, y } => { out(x); out(y); }
        E::None => out(0),
    }
}
