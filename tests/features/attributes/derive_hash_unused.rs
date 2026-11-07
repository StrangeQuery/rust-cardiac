//! feature: `#[derive(Hash)]` on a struct and an enum, never hashed
//! section: Derive
//! input: 3 4
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[derive(Hash, PartialEq, Eq)]
struct P { x: i16, y: i16 }
#[derive(Hash, PartialEq, Eq)]
enum E { A, B(i16) }
pub fn main() {
    let p = P { x: inp(), y: inp() };
    let e = if p.x > p.y { E::A } else { E::B(p.y) };
    out(match e { E::A => p.x, E::B(v) => v - p.x });
}
