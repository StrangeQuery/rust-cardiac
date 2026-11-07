//! feature: `.max()` and `.min()` on a `#[derive(Ord)]` struct
//! section: Derive
//! disposition: demand
//! note: The structs are passed whole to the derived `max`, which the compiler doesn't do yet.
//! input: 1 2 1 3 5 0 1 9
#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 { 0 }
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct P { x: i16, y: i16 }
pub fn main() {
    let a = P { x: inp(), y: inp() };
    let b = P { x: inp(), y: inp() };
    let m = a.max(b);
    out(m.x); out(m.y);
    let c = P { x: inp(), y: inp() };
    let d = P { x: inp(), y: inp() };
    let n = c.min(d);
    out(n.x); out(n.y);
}
