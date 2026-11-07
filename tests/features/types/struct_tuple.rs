//! feature: A tuple struct `struct Meters(i16)` passed and returned
//! section: Struct types
//! input: 3 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
struct Meters(i16);
#[inline(never)]
fn add(a: Meters, b: Meters) -> Meters {
    Meters(a.0 + b.0)
}
pub fn main() {
    let m = add(Meters(inp()), Meters(inp()));
    out(m.0);
}
