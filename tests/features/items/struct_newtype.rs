//! feature: A newtype struct (`struct Cents(i16)`)
//! section: Structs
//! input: 250 125
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[derive(Clone, Copy)]
struct Cents(i16);
#[inline(never)]
fn total(a: Cents, b: Cents) -> Cents { Cents(a.0 + b.0) }
pub fn main() {
    let t = total(Cents(inp()), Cents(inp()));
    out(t.0);
}
